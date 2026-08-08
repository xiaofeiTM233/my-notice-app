#pragma once
#include "pch.h"

namespace nm {

struct AppConfig {
    // Backend
    std::string wsUrl = "ws://localhost:8080/notifications";
    int reconnectInterval = 5;
    int maxReconnectAttempts = 10;

    // Notification display
    int popupDuration = 5000;
    int popupMaxCount = 3;
    int popupSpacing = 8;
    bool popupSound = false;

    // Notification center
    int maxHistory = 500;
    bool groupByDate = true;

    // Filters
    std::vector<std::string> categoryFilter;
    std::vector<std::string> priorityFilter;

    // UI
    bool startMinimized = false;
    bool closeToTray = true;
    bool autoStart = false;

    static AppConfig Load(const std::string& path);
    void Save(const std::string& path) const;
    static AppConfig Default();
};

} // namespace nm
