#pragma once
#include "pch.h"

struct BackendConfig
{
    enum class Mode { WebSocket, HttpLongPoll };
    Mode mode = Mode::WebSocket;
    std::wstring wsUrl;
    std::wstring httpUrl;
    int pollIntervalMs = 5000;
    std::wstring authToken;
    bool autoReconnect = true;
    int reconnectDelayMs = 3000;
};

struct PopupConfig
{
    int displayDurationMs = 5000;
    int maxVisible = 3;
    int animationDurationMs = 300;
    int width = 360;
    int marginRight = 20;
    int marginBottom = 20;
    int spacing = 12;
    bool showCloseButton = true;
    bool playSound = false;
};

struct FilterRule
{
    std::wstring category;
    std::wstring priority;
    std::wstring keyword;
    bool block = false;
    bool mute = false;
};

struct AppConfig
{
    BackendConfig backend;
    PopupConfig popup;
    std::vector<FilterRule> filters;
    bool startMinimized = false;
    bool minimizeToTray = true;
    bool closeToTray = true;
    std::wstring theme = L"dark"; // dark / light / system
    std::wstring language = L"zh-CN";
    int maxHistory = 500;

    winrt::JsonObject ToJson() const;
    static AppConfig FromJson(const winrt::JsonObject& obj);
};
