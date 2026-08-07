#include "Config.h"
#include "Util.h"

using namespace winrt::Windows::Data::Json;
using namespace winrt::Windows::Storage;

namespace winn
{
    std::wstring Config::DataPath() const
    {
        auto p = ApplicationData::Current().LocalFolder().Path();
        std::wstring dir = p;
        return dir + L"\\history.json";
    }

    void Config::Load()
    {
        std::string raw;
        if (ReadFile(ExeDir() + L"\\config.json", raw))
        {
            try
            {
                JsonObject o = JsonObject::Parse(winrt::to_hstring(raw));
                auto backend = o.GetNamedObject(L"backend", JsonObject());
                backendType = std::wstring(backend.GetNamedString(L"type", winrt::hstring{ backendType }));
                backendUrl = std::wstring(backend.GetNamedString(L"url", L""));
                pollIntervalSec = (int)backend.GetNamedNumber(L"pollIntervalSec", pollIntervalSec);

                auto popup = o.GetNamedObject(L"popup", JsonObject());
                popupEnabled = popup.GetNamedBoolean(L"enabled", popupEnabled);
                popupDurationSec = popup.GetNamedNumber(L"durationSec", popupDurationSec);
                popupWidth = popup.GetNamedNumber(L"width", popupWidth);
                popupMaxStack = (int)popup.GetNamedNumber(L"maxStack", popupMaxStack);

                auto sound = o.GetNamedObject(L"sound", JsonObject());
                soundEnabled = sound.GetNamedBoolean(L"enabled", soundEnabled);

                startMinimized = o.GetNamedBoolean(L"startMinimized", startMinimized);

                auto history = o.GetNamedObject(L"history", JsonObject());
                historyMax = (int)history.GetNamedNumber(L"maxItems", historyMax);

                mutedCategories.clear();
                auto muted = o.GetNamedArray(L"mutedCategories", JsonArray());
                for (auto&& v : muted)
                    mutedCategories.push_back(std::wstring(v.GetString()));

                filters.clear();
                auto fl = o.GetNamedArray(L"filters", JsonArray());
                for (auto&& v : fl)
                {
                    auto fo = v.GetObject();
                    FilterRule r;
                    r.pattern = std::wstring(fo.GetNamedString(L"pattern", L""));
                    r.enabled = fo.GetNamedBoolean(L"enabled", true);
                    r.block = fo.GetNamedBoolean(L"block", false);
                    if (!r.pattern.empty())
                        filters.push_back(std::move(r));
                }
            }
            catch (...) {}
        }
        else
        {
            Save();
        }
    }

    void Config::Save() const
    {
        JsonObject o;
        JsonObject backend;
        backend.SetNamedValue(L"type", JsonValue::CreateStringValue(winrt::hstring{ backendType }));
        backend.SetNamedValue(L"url", JsonValue::CreateStringValue(winrt::hstring{ backendUrl }));
        backend.SetNamedValue(L"pollIntervalSec", JsonValue::CreateNumberValue(pollIntervalSec));
        o.SetNamedValue(L"backend", backend);

        JsonObject popup;
        popup.SetNamedValue(L"enabled", JsonValue::CreateBooleanValue(popupEnabled));
        popup.SetNamedValue(L"durationSec", JsonValue::CreateNumberValue(popupDurationSec));
        popup.SetNamedValue(L"width", JsonValue::CreateNumberValue(popupWidth));
        popup.SetNamedValue(L"maxStack", JsonValue::CreateNumberValue(popupMaxStack));
        o.SetNamedValue(L"popup", popup);

        JsonObject sound;
        sound.SetNamedValue(L"enabled", JsonValue::CreateBooleanValue(soundEnabled));
        o.SetNamedValue(L"sound", sound);

        o.SetNamedValue(L"startMinimized", JsonValue::CreateBooleanValue(startMinimized));

        JsonObject history;
        history.SetNamedValue(L"maxItems", JsonValue::CreateNumberValue(historyMax));
        o.SetNamedValue(L"history", history);

        JsonArray muted;
        for (auto& c : mutedCategories)
            muted.Append(JsonValue::CreateStringValue(winrt::hstring{ c }));
        o.SetNamedValue(L"mutedCategories", muted);

        JsonArray fl;
        for (auto& r : filters)
        {
            JsonObject fo;
            fo.SetNamedValue(L"pattern", JsonValue::CreateStringValue(winrt::hstring{ r.pattern }));
            fo.SetNamedValue(L"enabled", JsonValue::CreateBooleanValue(r.enabled));
            fo.SetNamedValue(L"block", JsonValue::CreateBooleanValue(r.block));
            fl.Append(fo);
        }
        o.SetNamedValue(L"filters", fl);

        WriteFile(ExeDir() + L"\\config.json", WToUtf8(std::wstring(o.Stringify())));
    }

    bool Config::Pass(const NotifyItem& it) const
    {
        for (auto& c : mutedCategories)
        {
            if (it.cat == NotifyItem::CatFromName(c))
                return false;
        }
        bool hasAllow = std::any_of(filters.begin(), filters.end(),
            [](const FilterRule& r) { return r.enabled && !r.block; });
        bool allowMatch = false;
        for (auto& r : filters)
        {
            if (!r.enabled)
                continue;
            auto q = ToLower(r.pattern);
            bool m = ToLower(it.title).find(q) != std::wstring::npos
                || ToLower(it.body).find(q) != std::wstring::npos
                || ToLower(it.app).find(q) != std::wstring::npos;
            if (m && r.block)
                return false;
            if (m && !r.block)
                allowMatch = true;
        }
        if (hasAllow && !allowMatch)
            return false;
        return true;
    }
}
