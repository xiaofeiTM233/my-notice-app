#include "pch.h"
#include "AppConfig.h"

namespace nm {

AppConfig AppConfig::Default() {
    return AppConfig{};
}

AppConfig AppConfig::Load(const std::string& path) {
    namespace fs = std::filesystem;
    auto cfg = Default();
    auto hpath = winrt::to_hstring(path);

    try {
        auto file = winrt::Windows::Storage::StorageFile::GetFileFromPathAsync(hpath).get();
        auto content = winrt::Windows::Storage::FileIO::ReadTextAsync(file).get();
        auto json = winrt::Windows::Data::Json::JsonObject::Parse(content);

        auto getStr = [&](const wchar_t* k, const std::string& def = "") -> std::string {
            auto v = json.TryLookup(k);
            return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::String
                ? winrt::to_string(v.GetString()) : def;
        };
        auto getInt = [&](const wchar_t* k, int def = 0) -> int {
            auto v = json.TryLookup(k);
            return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::Number
                ? static_cast<int>(v.GetNumber()) : def;
        };
        auto getBool = [&](const wchar_t* k, bool def = false) -> bool {
            auto v = json.TryLookup(k);
            return v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::Boolean
                ? v.GetBoolean() : def;
        };
        auto getStrArr = [&](const wchar_t* k) -> std::vector<std::string> {
            std::vector<std::string> arr;
            auto v = json.TryLookup(k);
            if (v && v.ValueType() == winrt::Windows::Data::Json::JsonValueType::Array) {
                for (auto& elem : v.GetArray()) {
                    if (elem.ValueType() == winrt::Windows::Data::Json::JsonValueType::String)
                        arr.push_back(winrt::to_string(elem.GetString()));
                }
            }
            return arr;
        };

        cfg.wsUrl = getStr(L"wsUrl", cfg.wsUrl);
        cfg.reconnectInterval = getInt(L"reconnectInterval", cfg.reconnectInterval);
        cfg.maxReconnectAttempts = getInt(L"maxReconnectAttempts", cfg.maxReconnectAttempts);
        cfg.popupDuration = getInt(L"popupDuration", cfg.popupDuration);
        cfg.popupMaxCount = getInt(L"popupMaxCount", cfg.popupMaxCount);
        cfg.popupSpacing = getInt(L"popupSpacing", cfg.popupSpacing);
        cfg.popupSound = getBool(L"popupSound", cfg.popupSound);
        cfg.maxHistory = getInt(L"maxHistory", cfg.maxHistory);
        cfg.groupByDate = getBool(L"groupByDate", cfg.groupByDate);
        cfg.categoryFilter = getStrArr(L"categoryFilter");
        cfg.priorityFilter = getStrArr(L"priorityFilter");
        cfg.startMinimized = getBool(L"startMinimized", cfg.startMinimized);
        cfg.closeToTray = getBool(L"closeToTray", cfg.closeToTray);
        cfg.autoStart = getBool(L"autoStart", cfg.autoStart);
    }
    catch (...) {
        // Use defaults if config read fails
    }

    return cfg;
}

void AppConfig::Save(const std::string& path) const {
    using namespace winrt::Windows::Data::Json;
    JsonObject json;

    json.SetNamedValue(L"wsUrl", JsonValue::CreateString(winrt::to_hstring(wsUrl)));
    json.SetNamedValue(L"reconnectInterval", JsonValue::CreateNumberValue(reconnectInterval));
    json.SetNamedValue(L"maxReconnectAttempts", JsonValue::CreateNumberValue(maxReconnectAttempts));
    json.SetNamedValue(L"popupDuration", JsonValue::CreateNumberValue(popupDuration));
    json.SetNamedValue(L"popupMaxCount", JsonValue::CreateNumberValue(popupMaxCount));
    json.SetNamedValue(L"popupSpacing", JsonValue::CreateNumberValue(popupSpacing));
    json.SetNamedValue(L"popupSound", JsonValue::CreateBooleanValue(popupSound));
    json.SetNamedValue(L"maxHistory", JsonValue::CreateNumberValue(maxHistory));
    json.SetNamedValue(L"groupByDate", JsonValue::CreateBooleanValue(groupByDate));

    JsonArray catArr, priArr;
    for (auto& c : categoryFilter) catArr.Append(JsonValue::CreateString(winrt::to_hstring(c)));
    for (auto& p : priorityFilter) priArr.Append(JsonValue::CreateString(winrt::to_hstring(p)));
    json.SetNamedValue(L"categoryFilter", catArr);
    json.SetNamedValue(L"priorityFilter", priArr);

    json.SetNamedValue(L"startMinimized", JsonValue::CreateBooleanValue(startMinimized));
    json.SetNamedValue(L"closeToTray", JsonValue::CreateBooleanValue(closeToTray));
    json.SetNamedValue(L"autoStart", JsonValue::CreateBooleanValue(autoStart));

    try {
        auto hpath = winrt::to_hstring(path);
        auto folder = std::filesystem::path(path).parent_path();
        auto folderPath = winrt::to_hstring(folder.string());
        auto storageFolder = winrt::Windows::Storage::StorageFolder::GetFolderFromPathAsync(folderPath).get();
        auto file = storageFolder.CreateFileAsync(
            winrt::to_hstring(std::filesystem::path(path).filename().string()),
            winrt::Windows::Storage::CreationCollisionOption::ReplaceExisting).get();
        winrt::Windows::Storage::FileIO::WriteTextAsync(file, json.Stringify()).get();
    }
    catch (...) {}
}

} // namespace nm
