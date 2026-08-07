#pragma once
#include "App.xaml.g.h"
#include "MainWindow.xaml.h"
#include "core/Config.h"
#include "core/NotifyCenter.h"
#include "core/BackendClient.h"
#include "core/Toast.h"
#include "core/TrayIcon.h"

namespace winrt::WinNotify::implementation
{
    struct App : AppT<App>
    {
        App();
        void OnLaunched(winrt::Microsoft::UI::Xaml::LaunchActivatedEventArgs const&);

        static App& Instance();
        winn::Config& Cfg() { return m_cfg; }
        winn::NotifyCenter& Center() { return m_center; }
        winn::BackendClient& Backend() { return m_backend; }
        winn::ToastMgr& Toasts() { return m_toasts; }
        winn::TrayIcon& Tray() { return m_tray; }
        winrt::Microsoft::UI::Dispatching::DispatcherQueue UIQueue() { return m_queue; }
        bool Exiting() const { return m_exiting; }
        bool Paused() const { return m_paused; }

        void RequestExit();
        void TogglePause();
        void OnMainClosed();
        void OnNotif(const winn::NotifyItem& it);
        void HandleAction(const winn::NotifyItem& it);
        winrt::fire_and_forget OpenUrl(std::wstring url);

    private:
        winn::Config m_cfg;
        winn::NotifyCenter m_center;
        winn::BackendClient m_backend;
        winn::ToastMgr m_toasts;
        winn::TrayIcon m_tray;
        winrt::com_ptr<winrt::WinNotify::implementation::MainWindow> m_main{ nullptr };
        winrt::Microsoft::UI::Dispatching::DispatcherQueue m_queue{ nullptr };
        std::atomic<bool> m_paused{ false };
        std::atomic<bool> m_exiting{ false };
    };
}
namespace winrt::WinNotify::factory_implementation
{
    struct App : AppT<App, implementation::App>
    {
    };
}
