#pragma once
#include "pch.h"

namespace nm {

enum class NotifCategory { Message, Reminder, System, Custom };
enum class NotifPriority { Low, Normal, High, Urgent };

struct NotificationItem {
    std::string id;
    std::string title;
    std::string body;
    std::string icon;
    NotifCategory category = NotifCategory::Message;
    NotifPriority priority = NotifPriority::Normal;
    std::string actionUrl;
    std::string actionLabel;
    bool read = false;
    uint64_t timestamp = 0;

    static NotifCategory CategoryFromString(const std::string& s) {
        if (s == "reminder") return NotifCategory::Reminder;
        if (s == "system") return NotifCategory::System;
        if (s == "custom") return NotifCategory::Custom;
        return NotifCategory::Message;
    }

    static std::string CategoryToString(NotifCategory c) {
        switch (c) {
        case NotifCategory::Reminder: return "reminder";
        case NotifCategory::System: return "system";
        case NotifCategory::Custom: return "custom";
        default: return "message";
        }
    }

    static NotifPriority PriorityFromString(const std::string& s) {
        if (s == "low") return NotifPriority::Low;
        if (s == "high") return NotifPriority::High;
        if (s == "urgent") return NotifPriority::Urgent;
        return NotifPriority::Normal;
    }

    static winrt::Windows::Data::Json::JsonObject ToJson(const NotificationItem& item);
    static NotificationItem FromJson(const winrt::Windows::Data::Json::JsonObject& json);
};

} // namespace nm
