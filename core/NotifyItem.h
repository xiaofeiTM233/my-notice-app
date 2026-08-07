#pragma once
#include "pch.h"

namespace winn
{
    enum class Cat { Message, Reminder, System, Other };
    enum class Pri { Low, Normal, High };

    struct NotifyItem
    {
        std::wstring id;
        std::wstring title;
        std::wstring body;
        std::wstring app;
        Cat cat{ Cat::Other };
        Pri pri{ Pri::Normal };
        std::wstring actionUrl;
        std::wstring actionId;
        int64_t time{ 0 };
        bool read{ false };

        winrt::Windows::Data::Json::JsonObject ToJson() const;
        static NotifyItem FromJson(const winrt::Windows::Data::Json::JsonObject& o);
        static std::wstring CatName(Cat c);
        static Cat CatFromName(const std::wstring& s);
        static std::wstring PriName(Pri p);
        static Pri PriFromName(const std::wstring& s);
    };
}
