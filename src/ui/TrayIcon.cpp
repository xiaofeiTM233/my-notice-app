#include "pch.h"
#include "ui/TrayIcon.h"

enum TrayCmd
{
    CMD_SHOW = 1001,
    CMD_MARK_ALL_READ,
    CMD_CLEAR_ALL,
    CMD_SETTINGS,
    CMD_EXIT
};

TrayIcon::TrayIcon()
{
}

TrayIcon::~TrayIcon()
{
    Remove();
    if (m_hMenu) DestroyMenu(m_hMenu);
}

bool TrayIcon::Create(HWND hwnd, HICON icon, const std::wstring& tooltip)
{
    m_hwnd = hwnd;

    m_nid.cbSize = sizeof(NOTIFYICONDATA);
    m_nid.hWnd = hwnd;
    m_nid.uID = 1;
    m_nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    m_nid.uCallbackMessage = WM_TRAYMSG;
    m_nid.hIcon = icon;

    wcsncpy_s(m_nid.szTip, tooltip.c_str(), _TRUNCATE);

    BuildMenu();
    return Shell_NotifyIcon(NIM_ADD, &m_nid) == TRUE;
}

void TrayIcon::Remove()
{
    if (m_hwnd)
    {
        m_nid.uFlags = 0;
        Shell_NotifyIcon(NIM_DELETE, &m_nid);
        m_hwnd = nullptr;
    }
}

void TrayIcon::UpdateTooltip(const std::wstring& tooltip)
{
    if (!m_hwnd) return;
    m_nid.uFlags = NIF_TIP;
    wcsncpy_s(m_nid.szTip, tooltip.c_str(), _TRUNCATE);
    Shell_NotifyIcon(NIM_MODIFY, &m_nid);
}

void TrayIcon::ShowBalloon(const std::wstring& title, const std::wstring& msg, int timeoutMs)
{
    if (!m_hwnd) return;
    m_nid.uFlags = NIF_INFO;
    m_nid.uTimeout = timeoutMs;
    m_nid.dwInfoFlags = NIIF_INFO;
    wcsncpy_s(m_nid.szInfoTitle, title.c_str(), _TRUNCATE);
    wcsncpy_s(m_nid.szInfo, msg.c_str(), _TRUNCATE);
    Shell_NotifyIcon(NIM_MODIFY, &m_nid);
}

void TrayIcon::BuildMenu()
{
    m_hMenu = CreatePopupMenu();

    AppendMenu(m_hMenu, MF_STRING, CMD_SHOW, L"打开通知中心");
    AppendMenu(m_hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenu(m_hMenu, MF_STRING, CMD_MARK_ALL_READ, L"全部标为已读");
    AppendMenu(m_hMenu, MF_STRING, CMD_CLEAR_ALL, L"清空所有通知");
    AppendMenu(m_hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenu(m_hMenu, MF_STRING, CMD_SETTINGS, L"设置");
    AppendMenu(m_hMenu, MF_SEPARATOR, 0, nullptr);
    AppendMenu(m_hMenu, MF_STRING, CMD_EXIT, L"退出");
}

void TrayIcon::ShowContextMenu(int x, int y)
{
    if (!m_hMenu || !m_hwnd) return;

    SetForegroundWindow(m_hwnd);

    POINT pt = { x, y };
    TrackPopupMenu(m_hMenu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_RIGHTALIGN,
        pt.x, pt.y, 0, m_hwnd, nullptr);

    PostMessage(m_hwnd, WM_NULL, 0, 0);
}
