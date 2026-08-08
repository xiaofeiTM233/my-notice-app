#pragma once
#include "pch.h"

enum class NotifyCategory
{
    Message,
    Reminder,
    System,
    Alert,
    Custom
};

enum class NotifyPriority
{
    Low,
    Normal,
    High,
    Urgent
};

struct Notification
{
    std::wstring id;
    std::wstring title;
    std::wstring content;
    NotifyCategory category = NotifyCategory::Message;
    NotifyPriority priority = NotifyPriority::Normal;
    std::chrono::system_clock::time_point timestamp;
    bool read = false;
    std::wstring actionUrl;
    std::wstring groupKey;
    std::wstring iconPath;

    static std::wstring CategoryToString(NotifyCategory cat);
    static NotifyCategory StringToCategory(const std::wstring& s);
    static std::wstring PriorityToString(NotifyPriority pri);
    static NotifyPriority StringToPriority(const std::wstring& s);

    winrt::JsonObject ToJson() const;
    static Notification FromJson(const winrt::JsonObject& obj);

    std::wstring TimeString() const;
};
