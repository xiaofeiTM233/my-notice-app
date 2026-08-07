#pragma once
#include "pch.h"
#include "NotifyItem.h"

namespace winn
{
    struct FilterRule
    {
        std::wstring pattern;
        bool enabled{ true };
        bool block{ false };
    };

    struct Config
    {
        std::wstring backendType{ L"websocket" };
        std::wstring backendUrl;
        int pollIntervalSec{ 30 };

        bool popupEnabled{ true };
        double popupDurationSec{ 6.0 };
        double popupWidth{ 380.0 };
        int popupMaxStack{ 5 };

        bool soundEnabled{ false };
        bool startMinimized{ true };
        int historyMax{ 500 };

        std::vector<FilterRule> filters;
        std::vector<std::wstring> mutedCategories;

        void Load();
        void Save() const;
        bool Pass(const NotifyItem& it) const;
        std::wstring DataPath() const;
    };
}
