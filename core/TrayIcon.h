#pragma once
#include "pch.h"

namespace winn
{
    class TrayIcon
    {
    public:
        TrayIcon() = default;
        ~TrayIcon();

        void Init(HINSTANCE hinst);
        void Shutdown();
        void ShowBalloon(const std::wstring& title, const std::wstring& msg);
        void SetPauseLabel(bool paused);

        std::function<void()> onOpen;
        std::function<void()> onExit;
        std::function<void()> onTogglePause;

    private:
        static LRESULT CALLBACK WndProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp);
        void OnTray(UINT id);
        void ShowMenu();

        HWND m_hwnd = nullptr;
        bool m_added = false;
        bool m_paused = false;
        HINSTANCE m_hinst = nullptr;
    };
}
