#pragma once
#include "pch.h"
#include "MainWindow.g.h"
#include "ui/TrayIcon.h"

namespace winrt::WinNotify::implementation
{
    struct MainWindow : MainWindowT<MainWindow>
    {
        MainWindow();
        ~MainWindow();

        void MinimizeBtn_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);
        void CloseBtn_Click(winrt::Windows::Foundation::IInspectable const& sender,
            winrt::Microsoft::UI::Xaml::RoutedEventArgs const& e);

    private:
        std::unique_ptr<TrayIcon> m_trayIcon;
        HWND m_hwnd = nullptr;
        winrt::event_token m_sizeChangedToken;

        void SetupTitleBar();
        void SetupTrayIcon();
        void HideToTray();
        void ShowFromTray();

        static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wParam, LPARAM lParam);
        WNDPROC m_origWndProc = nullptr;

        void OnTrayMessage(WPARAM wParam, LPARAM lParam);
        void OnTrayMenuCmd(int cmd);
    };
}

namespace winrt::WinNotify::factory_implementation
{
    struct MainWindow : MainWindowT<MainWindow, implementation::MainWindow> {};
}
