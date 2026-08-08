#pragma once
#include "pch.h"
#include "ui/NotificationPopup.xaml.h"
#include "models/Notification.h"

class PopupWindow
{
public:
    PopupWindow();
    ~PopupWindow();

    void Show(const Notification& n, int x, int y, int animMs);
    void Hide(int animMs);
    void Move(int x, int y, int animMs);
    void SetAutoClose(int ms);

    winrt::event_token Closed(winrt::Windows::Foundation::EventHandler<winrt::hstring> const& handler)
    {
        return m_closed.add(handler);
    }
    void Closed(winrt::event_token const& token) noexcept { m_closed.remove(token); }

    winrt::event_token Clicked(winrt::Windows::Foundation::EventHandler<winrt::hstring> const& handler)
    {
        return m_clicked.add(handler);
    }
    void Clicked(winrt::event_token const& token) noexcept { m_clicked.remove(token); }

    bool IsVisible() const { return m_visible; }
    int Height() const { return m_height; }

private:
    void CreateWindow();
    void SetupAnimation();

    HWND m_hwnd = nullptr;
    winrt::Microsoft::UI::Xaml::Window m_window{nullptr};
    winrt::WinNotify::NotificationPopup m_popup{nullptr};

    winrt::event<winrt::Windows::Foundation::EventHandler<winrt::hstring>> m_closed;
    winrt::event<winrt::Windows::Foundation::EventHandler<winrt::hstring>> m_clicked;

    std::jthread m_autoCloseThread;
    std::atomic<bool> m_visible{false};
    int m_height = 100;
    int m_x = 0;
    int m_y = 0;
};

class PopupManager
{
public:
    static PopupManager& Instance();

    void Init();
    void ShowNotification(const Notification& n);
    void CloseAll();

private:
    PopupManager() = default;
    void Rearrange();
    void OnPopupClosed(const winrt::hstring& id);
    void OnPopupClicked(const winrt::hstring& id);

    std::vector<std::unique_ptr<PopupWindow>> m_popups;
    std::mutex m_mtx;
    int m_screenWidth = 0;
    int m_screenHeight = 0;
};
