#pragma once
#include "pch.h"

namespace nm {

class TrayIcon {
public:
    TrayIcon(HWND hwnd, HICON icon = nullptr);
    ~TrayIcon();

    void Show();
    void Hide();
    void SetIcon(HICON icon);
    void SetTooltip(const std::wstring& text);
    void ShowBalloon(const std::wstring& title, const std::wstring& msg);
    void UpdateUnreadBadge(int count);

    using MenuCallback = std::function<void(int cmdId)>;
    void SetMenuCallback(MenuCallback cb) { _onMenu = std::move(cb); }

    static constexpr UINT WM_TASKBAR = WM_APP + 100;
    enum MenuCmd { CMD_SHOW = 1, CMD_SETTINGS, CMD_EXIT, CMD_CLEAR_ALL };

private:
    void AddTray();
    void RemoveTray();

    HWND _hwnd;
    HICON _hIcon = nullptr;
    HICON _hDefaultIcon = nullptr;
    bool _visible = false;
    MenuCallback _onMenu;
    std::wstring _tooltip = L"Notification Manager";
};

} // namespace nm
