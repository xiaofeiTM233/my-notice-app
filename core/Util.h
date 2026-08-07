#pragma once
#include "pch.h"

namespace winn
{
    std::wstring Utf8ToW(const std::string& s);
    std::string WToUtf8(const std::wstring& s);
    std::wstring ToLower(std::wstring s);
    int64_t NowMs();
    std::wstring TimeStr(int64_t ms);
    bool ReadFile(const std::wstring& path, std::string& out);
    bool WriteFile(const std::wstring& path, const std::string& data);
    winrt::Windows::UI::Color ColorFromHex(uint32_t argb);
    std::wstring ExeDir();
}
