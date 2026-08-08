#include "pch.h"
#include "ui/NotificationCenter.xaml.h"
#include "services/NotificationService.h"
#if __has_include("NotificationCenter.g.cpp")
#include "NotificationCenter.g.cpp"
#endif

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Controls;
using namespace Windows::Foundation::Collections;

namespace winrt::WinNotify::implementation
{
    NotificationCenter::NotificationCenter()
    {
        InitializeComponent();
        m_items = single_threaded_observable_vector<IInspectable>();
        NotifyList().ItemsSource(m_items);
        RefreshList();
        UpdateUnreadBadge();
    }

    void NotificationCenter::RefreshList()
    {
        auto& svc = NotificationService::Instance();
        auto cat = FilterToCategory(m_currentFilter);
        bool onlyUnread = m_currentFilter == L"unread";

        auto list = svc.Search(m_searchKeyword, cat, onlyUnread);

        m_items.Clear();
        for (auto& n : list)
        {
            auto vm = winrt::make<NotifyItemVM>();
            vm->Id = n.id;
            vm->Title = n.title;
            vm->Content = n.content;

            switch (n.category)
            {
            case NotifyCategory::Message:  vm->Category = L"消息"; break;
            case NotifyCategory::Reminder: vm->Category = L"提醒"; break;
            case NotifyCategory::System:   vm->Category = L"系统"; break;
            case NotifyCategory::Alert:    vm->Category = L"告警"; break;
            default: vm->Category = L"通知"; break;
            }

            vm->Time = n.TimeString();
            vm->UnreadVis = n.read ? Visibility::Collapsed : Visibility::Visible;
            m_items.Append(vm);
        }

        // 空状态
        EmptyState().Visibility(list.empty() ? Visibility::Visible : Visibility::Collapsed);
        ListScroll().Visibility(list.empty() ? Visibility::Collapsed : Visibility::Visible);

        TotalCount().Text(std::format(L"共 {} 条", list.size()));
    }

    void NotificationCenter::UpdateUnreadBadge()
    {
        auto& svc = NotificationService::Instance();
        int cnt = svc.UnreadCount();

        UnreadCountText().Text(std::to_wstring(cnt));
        UnreadBadge().Visibility(cnt > 0 ? Visibility::Visible : Visibility::Collapsed);
    }

    void NotificationCenter::UpdateConnStatus(bool connected, const std::wstring& msg)
    {
        auto dispatcher = DispatcherQueue();
        dispatcher.TryEnqueue([this, connected, msg]() {
            ConnStatus().Text(msg);
            auto color = connected ? Windows::UI::ColorHelper::FromArgb(255, 76, 175, 80)
                                   : Windows::UI::ColorHelper::FromArgb(255, 244, 67, 54);
            ConnDot().Fill(SolidColorBrush(color));
        });
    }

    void NotificationCenter::SearchBox_QuerySubmitted(AutoSuggestBox const& sender,
        AutoSuggestBoxQuerySubmittedEventArgs const&)
    {
        m_searchKeyword = sender.Text().c_str();
        RefreshList();
    }

    void NotificationCenter::SearchBox_TextChanged(AutoSuggestBox const& sender,
        AutoSuggestBoxTextChangedEventArgs const&)
    {
        // 简单防抖：直接刷新（数据量不大）
        m_searchKeyword = sender.Text().c_str();
        RefreshList();
    }

    void NotificationCenter::Filter_Click(IInspectable const& sender, RoutedEventArgs const&)
    {
        auto btn = sender.as<Button>();
        auto tag = btn.Tag().as<winrt::hstring>().c_str();
        m_currentFilter = tag;
        SetActiveFilter(tag);
        RefreshList();
    }

    void NotificationCenter::SetActiveFilter(const std::wstring& tag)
    {
        auto res = Resources();
        auto activeStyle = res.Lookup(box_value(L"FilterBtnActive")).as<Style>();
        auto normalStyle = res.Lookup(box_value(L"FilterBtn")).as<Style>();

        std::vector<Button*> btns = { &BtnAll(), &BtnUnread(), &BtnMessage(),
                                      &BtnReminder(), &BtnSystem(), &BtnAlert() };
        for (auto* b : btns)
        {
            auto t = b->Tag().as<winrt::hstring>().c_str();
            b->Style(t == tag ? activeStyle : normalStyle);
        }
    }

    NotifyCategory NotificationCenter::FilterToCategory(const std::wstring& f)
    {
        if (f == L"message") return NotifyCategory::Message;
        if (f == L"reminder") return NotifyCategory::Reminder;
        if (f == L"system") return NotifyCategory::System;
        if (f == L"alert") return NotifyCategory::Alert;
        return static_cast<NotifyCategory>(-1);
    }

    void NotificationCenter::MarkAllRead_Click(IInspectable const&, RoutedEventArgs const&)
    {
        NotificationService::Instance().MarkAllRead();
        RefreshList();
        UpdateUnreadBadge();
    }

    void NotificationCenter::ClearAll_Click(IInspectable const&, RoutedEventArgs const&)
    {
        // 简单确认
        NotificationService::Instance().ClearAll();
        RefreshList();
        UpdateUnreadBadge();
    }

    void NotificationCenter::NotifyItem_Tapped(IInspectable const& sender, Input::TappedRoutedEventArgs const&)
    {
        auto border = sender.as<Border>();
        auto id = border.Tag().as<winrt::hstring>().c_str();

        auto& svc = NotificationService::Instance();
        svc.MarkRead(id);

        // 打开链接
        for (auto& n : svc.GetAll())
        {
            if (n.id == id && !n.actionUrl.empty())
            {
                Windows::System::Launcher::LaunchUriAsync(Windows::Foundation::Uri(n.actionUrl));
                break;
            }
        }

        RefreshList();
        UpdateUnreadBadge();
    }

    void NotificationCenter::DeleteBtn_Click(IInspectable const& sender, RoutedEventArgs const&)
    {
        auto btn = sender.as<Button>();
        auto id = btn.Tag().as<winrt::hstring>().c_str();
        NotificationService::Instance().RemoveNotification(id);
        RefreshList();
        UpdateUnreadBadge();
    }
}
