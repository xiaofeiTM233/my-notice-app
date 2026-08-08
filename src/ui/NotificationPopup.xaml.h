#pragma once
#include "pch.h"
#include "NotificationPopup.g.h"
#include "models/Notification.h"

namespace winrt::WinNotify::implementation
{
    struct NotificationPopup : NotificationPopupT<NotificationPopup>
    {
        NotificationPopup();

        void SetNotification(const Notification& n);

        winrt::hstring Title() { return m_title; }
        winrt::hstring Content() { return m_content; }
        winrt::hstring Category() { return m_category; }
        winrt::hstring TimeStr() { return m_timeStr; }

        void CloseBtn_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);

        winrt::event_token Closed(winrt::Windows::Foundation::EventHandler<winrt::hstring> const& handler)
        {
            return m_closedEvent.add(handler);
        }
        void Closed(winrt::event_token const& token) noexcept
        {
            m_closedEvent.remove(token);
        }

        winrt::event_token Clicked(winrt::Windows::Foundation::EventHandler<winrt::hstring> const& handler)
        {
            return m_clickedEvent.add(handler);
        }
        void Clicked(winrt::event_token const& token) noexcept
        {
            m_clickedEvent.remove(token);
        }

    private:
        winrt::hstring m_title;
        winrt::hstring m_content;
        winrt::hstring m_category;
        winrt::hstring m_timeStr;
        std::wstring m_notifyId;
        std::wstring m_actionUrl;

        winrt::event<winrt::Windows::Foundation::EventHandler<winrt::hstring>> m_closedEvent;
        winrt::event<winrt::Windows::Foundation::EventHandler<winrt::hstring>> m_clickedEvent;

        void OnTapped(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::Input::TappedRoutedEventArgs const& e);
    };
}

namespace winrt::WinNotify::factory_implementation
{
    struct NotificationPopup : NotificationPopupT<NotificationPopup, implementation::NotificationPopup> {};
}
