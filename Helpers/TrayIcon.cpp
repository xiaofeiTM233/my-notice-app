#include "pch.h"
#include "TrayIcon.h"

namespace nm {

TrayIcon::TrayIcon(HWND hwnd, HICON icon)
    : _hwnd(hwnd), _hDefaultIcon(icon) {
    _hIcon = icon;
}

TrayIcon::~TrayIcon() {
    Hide();
}

void TrayIcon::Show() {
    if (_visible) return;
    AddTray();
    _visible = true;
}

void TrayIcon::Hide() {
    if (!_visible) return;
    RemoveTray();
    _visible = false;
}

void TrayIcon::SetIcon(HICON icon) {
    _hIcon = icon;
    if (_visible) {
        NOTIFYICONDATAW nid = {};
        nid.cbSize = sizeof(nid);
        nid.hWnd = _hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON;
        nid.hIcon = icon;
        Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

void TrayIcon::SetTooltip(const std::wstring& text) {
    _tooltip = text;
    if (_visible) {
        NOTIFYICONDATAW nid = {};
        nid.cbSize = sizeof(nid);
        nid.hWnd = _hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_TIP;
        wcsncpy_s(nid.szTip, text.c_str(), _countof(nid.szTip) - 1);
        Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

void TrayIcon::ShowBalloon(const std::wstring& title, const std::wstring& msg) {
    if (!_visible) return;
    NOTIFYICONDATAW nid = {};
    nid.cbSize = sizeof(nid);
    nid.hWnd = _hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_INFO;
    nid.dwInfoFlags = NIIF_INFO;
    wcsncpy_s(nid.szInfoTitle, title.c_str(), _countof(nid.szInfoTitle) - 1);
    wcsncpy_s(nid.szInfo, msg.c_str(), _countof(nid.szInfo) - 1);
    Shell_NotifyIconW(NIM_MODIFY, &nid);
}

void TrayIcon::UpdateUnreadBadge(int count) {
    SetTooltip(L"Notification Manager" + (count > 0 ? L" (" + std::to_wstring(count) + L" unread)" : L""));
}

void TrayIcon::AddTray() {
    NOTIFYICONDATAW nid = {};
    nid.cbSize = sizeof(nid);
    nid.hWnd = _hwnd;
    nid.uID = 1;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TASKBAR;
    nid.hIcon = _hIcon ? _hIcon : LoadIconW(nullptr, IDI_APPLICATION);
    wcsncpy_s(nid.szTip, _tooltip.c_str(), _countof(nid.szTip) - 1);
    Shell_NotifyIconW(NIM_ADD, &nid);
}

void TrayIcon::RemoveTray() {
    NOTIFYICONDATAW nid = {};
    nid.cbSize = sizeof(nid);
    nid.hWnd = _hwnd;
    nid.uID = 1;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

} // namespace nm
