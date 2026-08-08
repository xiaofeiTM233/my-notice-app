#include "pch.h"
#include "models/Notification.h"

using namespace winrt;

std::wstring Notification::CategoryToString(NotifyCategory cat)
{
    switch (cat)
    {
    case NotifyCategory::Message:  return L"message";
    case NotifyCategory::Reminder: return L"reminder";
    case NotifyCategory::System:   return L"system";
    case NotifyCategory::Alert:    return L"alert";
    case NotifyCategory::Custom:   return L"custom";
    default: return L"message";
    }
}

NotifyCategory Notification::StringToCategory(const std::wstring& s)
{
    if (s == L"reminder") return NotifyCategory::Reminder;
    if (s == L"system")   return NotifyCategory::System;
    if (s == L"alert")    return NotifyCategory::Alert;
    if (s == L"custom")   return NotifyCategory::Custom;
    return NotifyCategory::Message;
}

std::wstring Notification::PriorityToString(NotifyPriority pri)
{
    switch (pri)
    {
    case NotifyPriority::Low:    return L"low";
    case NotifyPriority::Normal: return L"normal";
    case NotifyPriority::High:   return L"high";
    case NotifyPriority::Urgent: return L"urgent";
    default: return L"normal";
    }
}

NotifyPriority Notification::StringToPriority(const std::wstring& s)
{
    if (s == L"low")    return NotifyPriority::Low;
    if (s == L"high")   return NotifyPriority::High;
    if (s == L"urgent") return NotifyPriority::Urgent;
    return NotifyPriority::Normal;
}

JsonObject Notification::ToJson() const
{
    JsonObject obj;
    obj.SetNamedValue(L"id", JsonValue::CreateStringValue(id));
    obj.SetNamedValue(L"title", JsonValue::CreateStringValue(title));
    obj.SetNamedValue(L"content", JsonValue::CreateStringValue(content));
    obj.SetNamedValue(L"category", JsonValue::CreateStringValue(CategoryToString(category)));
    obj.SetNamedValue(L"priority", JsonValue::CreateStringValue(PriorityToString(priority)));
    obj.SetNamedValue(L"timestamp", JsonValue::CreateNumberValue(
        static_cast<double>(std::chrono::duration_cast<std::chrono::seconds>(
            timestamp.time_since_epoch()).count())));
    obj.SetNamedValue(L"read", JsonValue::CreateBooleanValue(read));
    obj.SetNamedValue(L"actionUrl", JsonValue::CreateStringValue(actionUrl));
    obj.SetNamedValue(L"groupKey", JsonValue::CreateStringValue(groupKey));
    obj.SetNamedValue(L"iconPath", JsonValue::CreateStringValue(iconPath));
    return obj;
}

Notification Notification::FromJson(const JsonObject& obj)
{
    Notification n;
    if (obj.HasKey(L"id")) n.id = obj.GetNamedString(L"id");
    if (obj.HasKey(L"title")) n.title = obj.GetNamedString(L"title");
    if (obj.HasKey(L"content")) n.content = obj.GetNamedString(L"content");
    if (obj.HasKey(L"category")) n.category = StringToCategory(obj.GetNamedString(L"category"));
    if (obj.HasKey(L"priority")) n.priority = StringToPriority(obj.GetNamedString(L"priority"));
    if (obj.HasKey(L"timestamp"))
    {
        auto secs = static_cast<int64_t>(obj.GetNamedNumber(L"timestamp"));
        n.timestamp = std::chrono::system_clock::time_point(std::chrono::seconds(secs));
    }
    else
    {
        n.timestamp = std::chrono::system_clock::now();
    }
    if (obj.HasKey(L"read")) n.read = obj.GetNamedBoolean(L"read");
    if (obj.HasKey(L"actionUrl")) n.actionUrl = obj.GetNamedString(L"actionUrl");
    if (obj.HasKey(L"groupKey")) n.groupKey = obj.GetNamedString(L"groupKey");
    if (obj.HasKey(L"iconPath")) n.iconPath = obj.GetNamedString(L"iconPath");
    return n;
}

std::wstring Notification::TimeString() const
{
    auto now = std::chrono::system_clock::now();
    auto diff = now - timestamp;
    auto mins = std::chrono::duration_cast<std::chrono::minutes>(diff).count();
    auto hours = std::chrono::duration_cast<std::chrono::hours>(diff).count();
    auto days = std::chrono::duration_cast<std::chrono::days>(diff).count();

    if (mins < 1) return L"刚刚";
    if (mins < 60) return std::format(L"{}分钟前", mins);
    if (hours < 24) return std::format(L"{}小时前", hours);
    if (days < 7) return std::format(L"{}天前", days);

    std::time_t t = std::chrono::system_clock::to_time_t(timestamp);
    std::tm tm{};
    localtime_s(&tm, &t);
    return std::format(L"{:04d}-{:02d}-{:02d}",
        tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday);
}
