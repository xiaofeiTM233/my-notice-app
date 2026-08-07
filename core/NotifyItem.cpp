#include "NotifyItem.h"

using namespace winrt::Windows::Data::Json;

namespace winn
{
    JsonObject NotifyItem::ToJson() const
    {
        JsonObject o;
        o.SetNamedValue(L"id", JsonValue::CreateStringValue(winrt::hstring{ id }));
        o.SetNamedValue(L"title", JsonValue::CreateStringValue(winrt::hstring{ title }));
        o.SetNamedValue(L"body", JsonValue::CreateStringValue(winrt::hstring{ body }));
        o.SetNamedValue(L"app", JsonValue::CreateStringValue(winrt::hstring{ app }));
        o.SetNamedValue(L"cat", JsonValue::CreateStringValue(winrt::hstring{ CatName(cat) }));
        o.SetNamedValue(L"pri", JsonValue::CreateStringValue(winrt::hstring{ PriName(pri) }));
        o.SetNamedValue(L"actionUrl", JsonValue::CreateStringValue(winrt::hstring{ actionUrl }));
        o.SetNamedValue(L"actionId", JsonValue::CreateStringValue(winrt::hstring{ actionId }));
        o.SetNamedValue(L"time", JsonValue::CreateNumberValue((double)time));
        o.SetNamedValue(L"read", JsonValue::CreateBooleanValue(read));
        return o;
    }

    NotifyItem NotifyItem::FromJson(const JsonObject& o)
    {
        NotifyItem it;
        it.id = std::wstring(o.GetNamedString(L"id", L""));
        it.title = std::wstring(o.GetNamedString(L"title", L""));
        it.body = std::wstring(o.GetNamedString(L"body", L""));
        it.app = std::wstring(o.GetNamedString(L"app", L""));
        it.cat = CatFromName(std::wstring(o.GetNamedString(L"cat", L"")));
        it.pri = PriFromName(std::wstring(o.GetNamedString(L"pri", L"")));
        it.actionUrl = std::wstring(o.GetNamedString(L"actionUrl", L""));
        it.actionId = std::wstring(o.GetNamedString(L"actionId", L""));
        it.time = (int64_t)o.GetNamedNumber(L"time", 0);
        it.read = o.GetNamedBoolean(L"read", false);
        if (it.id.empty())
            it.id = std::to_wstring(it.time);
        return it;
    }

    std::wstring NotifyItem::CatName(Cat c)
    {
        switch (c)
        {
        case Cat::Message: return L"message";
        case Cat::Reminder: return L"reminder";
        case Cat::System: return L"system";
        default: return L"other";
        }
    }

    Cat NotifyItem::CatFromName(const std::wstring& s)
    {
        if (s == L"message") return Cat::Message;
        if (s == L"reminder") return Cat::Reminder;
        if (s == L"system") return Cat::System;
        return Cat::Other;
    }

    std::wstring NotifyItem::PriName(Pri p)
    {
        switch (p)
        {
        case Pri::Low: return L"low";
        case Pri::High: return L"high";
        default: return L"normal";
        }
    }

    Pri NotifyItem::PriFromName(const std::wstring& s)
    {
        if (s == L"low") return Pri::Low;
        if (s == L"high") return Pri::High;
        return Pri::Normal;
    }
}
