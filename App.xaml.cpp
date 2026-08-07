#include "pch.h"
#include "App.xaml.h"
#include "core/Util.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Dispatching;
using namespace winrt::Windows::System;

namespace winrt::WinNotify::implementation
{
    static App* s_instance = nullptr;

    App::App()
    {
        s_instance = this;
#if defined _DEBUG && !defined DISABLE_XAML_GENERATED_BREAK_ON_UNHANDLED_EXCEPTION
        UnhandledException([](IInspectable const&, UnhandledExceptionEventArgs const& e) {
            if (IsDebuggerPresent())
            {
                auto msg = e.Message();
                __debugbreak();
            }
        });
#endif
    }

    App& App::Instance()
    {
        return *s_instance;
    }

    void App::OnLaunched(LaunchActivatedEventArgs const&)
    {
        m_queue = DispatcherQueue::GetForCurrentThread();
        m_cfg.Load();

        m_center.onChanged = [this] {
            if (m_queue)
                m_queue.TryEnqueue([this] {
                    if (m_main)
                        m_main->Refresh();
                });
        };

        m_backend.onNotif = [this](const winn::NotifyItem& it) {
            if (m_queue)
                m_queue.TryEnqueue([this, it] { OnNotif(it); });
        };

        m_tray.Init(GetModuleHandleW(nullptr));
        m_tray.onOpen = [this] {
            if (m_queue)
                m_queue.TryEnqueue([this] {
                    if (m_main)
                        m_main->ShowWindow();
                });
        };
        m_tray.onExit = [this] {
            if (m_queue)
                m_queue.TryEnqueue([this] { RequestExit(); });
        };
        m_tray.onTogglePause = [this] {
            if (m_queue)
                m_queue.TryEnqueue([this] { TogglePause(); });
        };

        m_main = winrt::make_self<winrt::WinNotify::implementation::MainWindow>();

        m_backend.Start();

        if (!m_cfg.startMinimized)
            m_main->ShowWindow();
    }

    void App::OnNotif(const winn::NotifyItem& it)
    {
        m_center.Add(it);

        if (m_cfg.popupEnabled)
        {
            m_toasts.Push(it, [this](const winn::NotifyItem& x) { HandleAction(x); });
        }

        if (m_cfg.soundEnabled)
            MessageBeep(MB_OK);
    }

    void App::HandleAction(const winn::NotifyItem& it)
    {
        if (!it.actionUrl.empty())
            OpenUrl(it.actionUrl);
    }

    winrt::fire_and_forget App::OpenUrl(std::wstring url)
    {
        try
        {
            co_await Launcher::LaunchUriAsync(
                winrt::Windows::Foundation::Uri{ winrt::hstring{ url } });
        }
        catch (...) {}
    }

    void App::TogglePause()
    {
        m_paused = !m_paused;
        m_backend.Pause(m_paused);
        m_tray.SetPauseLabel(m_paused);
        if (m_main)
            m_main->Refresh();
    }

    void App::RequestExit()
    {
        if (m_exiting)
            return;
        m_exiting = true;
        m_backend.Stop();
        m_toasts.CloseAll();
        m_tray.Shutdown();
        if (m_main)
            m_main->Close();
        else
            OnMainClosed();
    }

    void App::OnMainClosed()
    {
        if (m_exiting)
        {
            if (auto app = Application::Current())
                app.Exit();
        }
    }
}
