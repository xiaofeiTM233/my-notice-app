#pragma once
#include "pch.h"
#include "models/AppConfig.h"

class ConfigManager
{
public:
    static ConfigManager& Instance();

    void Load(const std::wstring& path);
    void Save();
    const AppConfig& Get() const { return m_config; }
    AppConfig& Mutable() { return m_config; }
    void Update(const AppConfig& cfg);

    std::wstring ConfigPath() const { return m_path; }

private:
    ConfigManager() = default;
    AppConfig m_config;
    std::wstring m_path;
    std::mutex m_mtx;
};
