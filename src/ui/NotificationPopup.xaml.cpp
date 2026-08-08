#include "pch.h"
#include "ui/NotificationPopup.xaml.h"
#if __has_include("NotificationPopup.g.cpp")
#include "NotificationPopup.g.cpp"
#endif

using namespace winrt;
using namespace Microsoft::UI::Xaml;
using namespace Microsoft::UI::Xaml::Media;

namespace winrt::WinNotify::implementation
{
    NotificationPopup::NotificationPopup()
    {
        InitializeComponent();

        RootBorder().Tapped({ this, &NotificationPopup::OnTapped });

        // 悬停效果
        RootBorder().PointerEntered([](auto&&, auto&&) {
            // 可以加一些悬停效果
        });
    }

    void NotificationPopup::SetNotification(const Notification& n)
    {
        m_title = n.title;
        m_content = n.content;
        m_notifyId = n.id;
        m_actionUrl = n.actionUrl;

        // 分类显示名
        switch (n.category)
        {
        case NotifyCategory::Message:  m_category = L"消息"; break;
        case NotifyCategory::Reminder: m_category = L"提醒"; break;
        case NotifyCategory::System:   m_category = L"系统"; break;
        case NotifyCategory::Alert:    m_category = L"告警"; break;
        default: m_category = L"通知"; break;
        }

        m_timeStr = n.TimeString();

        // 优先级颜色
        auto res = Resources();
        switch (n.priority)
        {
        case NotifyPriority::Urgent:
            PriorityBar().Background(res.Lookup(winrt::box_value(L"PriUrgent")).as<Brush>());
            break;
        case NotifyPriority::High:
            PriorityBar().Background(res.Lookup(winrt::box_value(L"PriHigh")).as<Brush>());
            break;
        case NotifyPriority::Low:
            PriorityBar().Background(res.Lookup(winrt::box_value(L"PriLow")).as<Brush>());
            break;
        default:
            PriorityBar().Background(res.Lookup(winrt::box_value(L"PriNormal")).as<Brush>());
            break;
        }

        Bindings->Update();
    }

    void NotificationPopup::CloseBtn_Click(IInspectable const&, RoutedEventArgs const&)
    {
        m_closedEvent(*this, m_notifyId);
    }

    void NotificationPopup::OnTapped(IInspectable const&, Input::TappedRoutedEventArgs const&)
    {
        m_clickedEvent(*this, m_notifyId);
    }
}
