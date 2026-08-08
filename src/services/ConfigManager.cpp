#include "pch.h"
#include "services/ConfigManager.h"

using namespace winrt;

ConfigManager& ConfigManager::Instance()
{
    static ConfigManager inst;
    return inst;
}

JsonObject AppConfig::ToJson() const
{
    JsonObject root;

    // backend
    JsonObject be;
    be.SetNamedValue(L"mode", JsonValue::CreateStringValue(
        backend.mode == BackendConfig::Mode::WebSocket ? L"websocket" : L"http_longpoll"));
    be.SetNamedValue(L"wsUrl", JsonValue::CreateStringValue(backend.wsUrl));
    be.SetNamedValue(L"httpUrl", JsonValue::CreateStringValue(backend.httpUrl));
    be.SetNamedValue(L"pollIntervalMs", JsonValue::CreateNumberValue(backend.pollIntervalMs));
    be.SetNamedValue(L"authToken", JsonValue::CreateStringValue(backend.authToken));
    be.SetNamedValue(L"autoReconnect", JsonValue::CreateBooleanValue(backend.autoReconnect));
    be.SetNamedValue(L"reconnectDelayMs", JsonValue::CreateNumberValue(backend.reconnectDelayMs));
    root.SetNamedValue(L"backend", be);

    // popup
    JsonObject pp;
    pp.SetNamedValue(L"displayDurationMs", JsonValue::CreateNumberValue(popup.displayDurationMs));
    pp.SetNamedValue(L"maxVisible", JsonValue::CreateNumberValue(popup.maxVisible));
    pp.SetNamedValue(L"animationDurationMs", JsonValue::CreateNumberValue(popup.animationDurationMs));
    pp.SetNamedValue(L"width", JsonValue::CreateNumberValue(popup.width));
    pp.SetNamedValue(L"marginRight", JsonValue::CreateNumberValue(popup.marginRight));
    pp.SetNamedValue(L"marginBottom", JsonValue::CreateNumberValue(popup.marginBottom));
    pp.SetNamedValue(L"spacing", JsonValue::CreateNumberValue(popup.spacing));
    pp.SetNamedValue(L"showCloseButton", JsonValue::CreateBooleanValue(popup.showCloseButton));
    pp.SetNamedValue(L"playSound", JsonValue::CreateBooleanValue(popup.playSound));
    root.SetNamedValue(L"popup", pp);

    // filters
    JsonArray farr;
    for (auto& f : filters)
    {
        JsonObject fo;
        fo.SetNamedValue(L"category", JsonValue::CreateStringValue(f.category));
        fo.SetNamedValue(L"priority", JsonValue::CreateStringValue(f.priority));
        fo.SetNamedValue(L"keyword", JsonValue::CreateStringValue(f.keyword));
        fo.SetNamedValue(L"block", JsonValue::CreateBooleanValue(f.block));
        fo.SetNamedValue(L"mute", JsonValue::CreateBooleanValue(f.mute));
        farr.Append(fo);
    }
    root.SetNamedValue(L"filters", farr);

    root.SetNamedValue(L"startMinimized", JsonValue::CreateBooleanValue(startMinimized));
    root.SetNamedValue(L"minimizeToTray", JsonValue::CreateBooleanValue(minimizeToTray));
    root.SetNamedValue(L"closeToTray", JsonValue::CreateBooleanValue(closeToTray));
    root.SetNamedValue(L"theme", JsonValue::CreateStringValue(theme));
    root.SetNamedValue(L"language", JsonValue::CreateStringValue(language));
    root.SetNamedValue(L"maxHistory", JsonValue::CreateNumberValue(maxHistory));

    return root;
}

AppConfig AppConfig::FromJson(const JsonObject& obj)
{
    AppConfig cfg;

    if (obj.HasKey(L"backend"))
    {
        auto be = obj.GetNamedObject(L"backend");
        if (be.HasKey(L"mode"))
            cfg.backend.mode = be.GetNamedString(L"mode") == L"http_longpoll"
                ? BackendConfig::Mode::HttpLongPoll : BackendConfig::Mode::WebSocket;
        if (be.HasKey(L"wsUrl")) cfg.backend.wsUrl = be.GetNamedString(L"wsUrl");
        if (be.HasKey(L"httpUrl")) cfg.backend.httpUrl = be.GetNamedString(L"httpUrl");
        if (be.HasKey(L"pollIntervalMs")) cfg.backend.pollIntervalMs = static_cast<int>(be.GetNamedNumber(L"pollIntervalMs"));
        if (be.HasKey(L"authToken")) cfg.backend.authToken = be.GetNamedString(L"authToken");
        if (be.HasKey(L"autoReconnect")) cfg.backend.autoReconnect = be.GetNamedBoolean(L"autoReconnect");
        if (be.HasKey(L"reconnectDelayMs")) cfg.backend.reconnectDelayMs = static_cast<int>(be.GetNamedNumber(L"reconnectDelayMs"));
    }

    if (obj.HasKey(L"popup"))
    {
        auto pp = obj.GetNamedObject(L"popup");
        if (pp.HasKey(L"displayDurationMs")) cfg.popup.displayDurationMs = static_cast<int>(pp.GetNamedNumber(L"displayDurationMs"));
        if (pp.HasKey(L"maxVisible")) cfg.popup.maxVisible = static_cast<int>(pp.GetNamedNumber(L"maxVisible"));
        if (pp.HasKey(L"animationDurationMs")) cfg.popup.animationDurationMs = static_cast<int>(pp.GetNamedNumber(L"animationDurationMs"));
        if (pp.HasKey(L"width")) cfg.popup.width = static_cast<int>(pp.GetNamedNumber(L"width"));
        if (pp.HasKey(L"marginRight")) cfg.popup.marginRight = static_cast<int>(pp.GetNamedNumber(L"marginRight"));
        if (pp.HasKey(L"marginBottom")) cfg.popup.marginBottom = static_cast<int>(pp.GetNamedNumber(L"marginBottom"));
        if (pp.HasKey(L"spacing")) cfg.popup.spacing = static_cast<int>(pp.GetNamedNumber(L"spacing"));
        if (pp.HasKey(L"showCloseButton")) cfg.popup.showCloseButton = pp.GetNamedBoolean(L"showCloseButton");
        if (pp.HasKey(L"playSound")) cfg.popup.playSound = pp.GetNamedBoolean(L"playSound");
    }

    if (obj.HasKey(L"filters"))
    {
        auto farr = obj.GetNamedArray(L"filters");
        for (uint32_t i = 0; i < farr.Size(); i++)
        {
            auto fo = farr.GetAt(i).GetObjectW();
            FilterRule f;
            if (fo.HasKey(L"category")) f.category = fo.GetNamedString(L"category");
            if (fo.HasKey(L"priority")) f.priority = fo.GetNamedString(L"priority");
            if (fo.HasKey(L"keyword")) f.keyword = fo.GetNamedString(L"keyword");
            if (fo.HasKey(L"block")) f.block = fo.GetNamedBoolean(L"block");
            if (fo.HasKey(L"mute")) f.mute = fo.GetNamedBoolean(L"mute");
            cfg.filters.push_back(f);
        }
    }

    if (obj.HasKey(L"startMinimized")) cfg.startMinimized = obj.GetNamedBoolean(L"startMinimized");
    if (obj.HasKey(L"minimizeToTray")) cfg.minimizeToTray = obj.GetNamedBoolean(L"minimizeToTray");
    if (obj.HasKey(L"closeToTray")) cfg.closeToTray = obj.GetNamedBoolean(L"closeToTray");
    if (obj.HasKey(L"theme")) cfg.theme = obj.GetNamedString(L"theme");
    if (obj.HasKey(L"language")) cfg.language = obj.GetNamedString(L"language");
    if (obj.HasKey(L"maxHistory")) cfg.maxHistory = static_cast<int>(obj.GetNamedNumber(L"maxHistory"));

    return cfg;
}

void ConfigManager::Load(const std::wstring& path)
{
    std::lock_guard lock(m_mtx);
    m_path = path;

    try
    {
        std::ifstream file(path);
        if (!file.is_open()) return;

        std::stringstream ss;
        ss << file.rdbuf();
        std::string content = ss.str();

        JsonObject obj;
        if (JsonObject::Parse(winrt::to_hstring(content), obj))
        {
            m_config = AppConfig::FromJson(obj);
        }
    }
    catch (...) {}
}

void ConfigManager::Save()
{
    std::lock_guard lock(m_mtx);
    if (m_path.empty()) return;

    try
    {
        auto json = m_config.ToJson();
        auto str = json.Stringify();
        std::ofstream file(m_path);
        file << winrt::to_string(str);
    }
    catch (...) {}
}

void ConfigManager::Update(const AppConfig& cfg)
{
    std::lock_guard lock(m_mtx);
    m_config = cfg;
    Save();
}
