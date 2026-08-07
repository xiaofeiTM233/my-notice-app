#include "TrayIcon.h"
#include "resource.h"

namespace winn
{
    static const UINT WM_TRAY = WM_APP + 1;
    static const UINT TRAY_ICON = 1;
    static TrayIcon* g_tray = nullptr;

    TrayIcon::~TrayIcon()
    {
        Shutdown();
    }

    void TrayIcon::Init(HINSTANCE hinst)
    {
        m_hinst = hinst;
        g_tray = this;

        WNDCLASSEXW wc{};
        wc.cbSize = sizeof(wc);
        wc.lpfnWndProc = &TrayIcon::WndProc;
        wc.hInstance = hinst;
        wc.lpszClassName = L"WinNotifyTray";
        RegisterClassExW(&wc);

        m_hwnd = CreateWindowExW(0, L"WinNotifyTray", L"", 0, 0, 0, 0, 0, nullptr, nullptr, hinst, nullptr);

        HICON icon = (HICON)LoadImageW(hinst, MAKEINTRESOURCEW(IDI_APP), IMAGE_ICON, 0, 0, LR_DEFAULTSIZE);
        NOTIFYICONDATAW nid{};
        nid.cbSize = sizeof(nid);
        nid.hWnd = m_hwnd;
        nid.uID = TRAY_ICON;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = WM_TRAY;
        nid.hIcon = icon;
        wcscpy_s(nid.szTip, L"WinNotify");
        Shell_NotifyIconW(NIM_ADD, &nid);
        m_added = true;
    }

    void TrayIcon::Shutdown()
    {
        if (m_added)
        {
            NOTIFYICONDATAW nid{};
            nid.cbSize = sizeof(nid);
            nid.hWnd = m_hwnd;
            nid.uID = TRAY_ICON;
            Shell_NotifyIconW(NIM_DELETE, &nid);
            m_added = false;
        }
        if (m_hwnd)
        {
            DestroyWindow(m_hwnd);
            m_hwnd = nullptr;
        }
    }

    void TrayIcon::ShowBalloon(const std::wstring& title, const std::wstring& msg)
    {
        if (!m_added)
            return;
        NOTIFYICONDATAW nid{};
        nid.cbSize = sizeof(nid);
        nid.hWnd = m_hwnd;
        nid.uID = TRAY_ICON;
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_INFO;
        wcscpy_s(nid.szInfoTitle, title.c_str());
        wcscpy_s(nid.szInfo, msg.c_str());
        Shell_NotifyIconW(NIM_MODIFY, &nid);
    }

    void TrayIcon::SetPauseLabel(bool paused)
    {
        m_paused = paused;
    }

    LRESULT CALLBACK TrayIcon::WndProc(HWND hwnd, UINT msg, WPARAM wp, LPARAM lp)
    {
        if (msg == WM_TRAY && g_tray)
        {
            g_tray->OnTray((UINT)lp);
            return 0;
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    }

    void TrayIcon::OnTray(UINT id)
    {
        if (id != TRAY_ICON)
            return;
        if (GetAsyncKeyState(VK_LBUTTON) & 0x8000)
        {
            if (onOpen)
                onOpen();
        }
        else if (GetAsyncKeyState(VK_RBUTTON) & 0x8000)
        {
            ShowMenu();
        }
    }

    void TrayIcon::ShowMenu()
    {
        HMENU menu = CreatePopupMenu();
        AppendMenuW(menu, MF_STRING, 1, L"打开通知中心");
        AppendMenuW(menu, MF_STRING, 2, m_paused ? L"恢复接收" : L"暂停接收");
        AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
        AppendMenuW(menu, MF_STRING, 3, L"退出");

        POINT pt;
        GetCursorPos(&pt);
        SetForegroundWindow(m_hwnd);
        int cmd = TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON, pt.x, pt.y, 0, m_hwnd, nullptr);
        DestroyMenu(menu);

        switch (cmd)
        {
        case 1:
            if (onOpen)
                onOpen();
            break;
        case 2:
            if (onTogglePause)
                onTogglePause();
            break;
        case 3:
            if (onExit)
                onExit();
            break;
        }
    }
}
