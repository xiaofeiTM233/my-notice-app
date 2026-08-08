#pragma once
#include "pch.h"
#include <shellapi.h>

class TrayIcon
{
public:
    using MenuCallback = std::function<void(int cmd)>;

    TrayIcon();
    ~TrayIcon();

    bool Create(HWND hwnd, HICON icon, const std::wstring& tooltip);
    void Remove();
    void UpdateTooltip(const std::wstring& tooltip);
    void ShowBalloon(const std::wstring& title, const std::wstring& msg, int timeoutMs = 3000);

    void SetMenuCallback(MenuCallback cb) { m_onMenu = std::move(cb); }
    void ShowContextMenu(int x, int y);

    static constexpr int WM_TRAYMSG = WM_USER + 100;

private:
    NOTIFYICONDATA m_nid{};
    HWND m_hwnd = nullptr;
    MenuCallback m_onMenu;
    HMENU m_hMenu = nullptr;

    void BuildMenu();
};
