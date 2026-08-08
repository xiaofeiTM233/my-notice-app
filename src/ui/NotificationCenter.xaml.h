#pragma once
#include "pch.h"
#include "NotificationCenter.g.h"
#include "models/Notification.h"

namespace winrt::WinNotify::implementation
{
    struct NotifyItemVM : winrt::implements<NotifyItemVM, winrt::Windows::Foundation::IInspectable>
    {
        winrt::hstring Id;
        winrt::hstring Title;
        winrt::hstring Content;
        winrt::hstring Category;
        winrt::hstring Time;
        winrt::Microsoft::UI::Xaml::Visibility UnreadVis;
    };

    struct NotificationCenter : NotificationCenterT<NotificationCenter>
    {
        NotificationCenter();

        void RefreshList();
        void UpdateUnreadBadge();
        void UpdateConnStatus(bool connected, const std::wstring& msg);

        void SearchBox_QuerySubmitted(winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const& sender,
            winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxQuerySubmittedEventArgs const& args);
        void SearchBox_TextChanged(winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBox const& sender,
            winrt::Microsoft::UI::Xaml::Controls::AutoSuggestBoxTextChangedEventArgs const& args);
        void Filter_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);
        void MarkAllRead_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);
        void ClearAll_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);
        void NotifyItem_Tapped(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::Input::TappedRoutedEventArgs const& e);
        void DeleteBtn_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);

    private:
        std::wstring m_currentFilter = L"all";
        std::wstring m_searchKeyword;

        void SetActiveFilter(const std::wstring& tag);
        NotifyCategory FilterToCategory(const std::wstring& f);
        winrt::Windows::Foundation::Collections::IVector<winrt::Windows::Foundation::IInspectable> m_items{ nullptr };
    };
}

namespace winrt::WinNotify::factory_implementation
{
    struct NotificationCenter : NotificationCenterT<NotificationCenter, implementation::NotificationCenter> {};
}
