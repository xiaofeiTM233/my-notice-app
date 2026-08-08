#include "pch.h"
#include "NotificationItem.h"

namespace nm {

winrt::Windows::Data::Json::JsonObject NotificationItem::ToJson(const NotificationItem& item) {
    using namespace winrt::Windows::Data::Json;
    JsonObject json;
    json.SetNamedValue(L"id", JsonValue::CreateString(winrt::to_hstring(item.id)));
    json.SetNamedValue(L"title", JsonValue::CreateString(winrt::to_hstring(item.title)));
    json.SetNamedValue(L"body", JsonValue::CreateString(winrt::to_hstring(item.body)));
    json.SetNamedValue(L"icon", JsonValue::CreateString(winrt::to_hstring(item.icon)));
    json.SetNamedValue(L"category", JsonValue::CreateString(winrt::to_hstring(CategoryToString(item.category))));
    json.SetNamedValue(L"priority", JsonValue::CreateString(winrt::to_hstring(
        item.priority == NotifPriority::Low ? "low" :
        item.priority == NotifPriority::High ? "high" :
        item.priority == NotifPriority::Urgent ? "urgent" : "normal")));
    json.SetNamedValue(L"actionUrl", JsonValue::CreateString(winrt::to_hstring(item.actionUrl)));
    json.SetNamedValue(L"actionLabel", JsonValue::CreateString(winrt::to_hstring(item.actionLabel)));
    json.SetNamedValue(L"read", JsonValue::CreateBooleanValue(item.read));
    json.SetNamedValue(L"timestamp", JsonValue::CreateNumberValue(static_cast<double>(item.timestamp)));
    return json;
}

NotificationItem NotificationItem::FromJson(const winrt::Windows::Data::Json::JsonObject& json) {
    NotificationItem item;
    auto getStr = [&](const wchar_t* key) -> std::string {
        auto v = json.TryLookup(key);
        return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::String
            ? winrt::to_string(v.GetString()) : "";
    };
    auto getBool = [&](const wchar_t* key, bool def = false) -> bool {
        auto v = json.TryLookup(key);
        return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::Boolean
            ? v.GetBoolean() : def;
    };
    auto getNum = [&](const wchar_t* key, double def = 0) -> double {
        auto v = json.TryLookup(key);
        return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::Number
            ? v.GetNumber() : def;
    };

    item.id = getStr(L"id");
    item.title = getStr(L"title");
    item.body = getStr(L"body");
    item.icon = getStr(L"icon");
    item.category = CategoryFromString(getStr(L"category"));
    item.priority = PriorityFromString(getStr(L"priority"));
    item.actionUrl = getStr(L"actionUrl");
    item.actionLabel = getStr(L"actionLabel");
    item.read = getBool(L"read");
    item.timestamp = static_cast<uint64_t>(getNum(L"timestamp"));
    return item;
}

} // namespace nm
