#include "Util.h"

namespace winn
{
    std::wstring Utf8ToW(const std::string& s)
    {
        if (s.empty())
            return {};
        int n = MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), nullptr, 0);
        std::wstring w(n, 0);
        MultiByteToWideChar(CP_UTF8, 0, s.data(), (int)s.size(), w.data(), n);
        return w;
    }

    std::string WToUtf8(const std::wstring& s)
    {
        if (s.empty())
            return {};
        int n = WideCharToMultiByte(CP_UTF8, 0, s.data(), (int)s.size(), nullptr, 0, nullptr, nullptr);
        std::string a(n, 0);
        WideCharToMultiByte(CP_UTF8, 0, s.data(), (int)s.size(), a.data(), n, nullptr, nullptr);
        return a;
    }

    std::wstring ToLower(std::wstring s)
    {
        std::transform(s.begin(), s.end(), s.begin(), [](wchar_t c) { return (wchar_t)towlower(c); });
        return s;
    }

    int64_t NowMs()
    {
        using namespace std::chrono;
        return duration_cast<milliseconds>(system_clock::now().time_since_epoch()).count();
    }

    std::wstring TimeStr(int64_t ms)
    {
        SYSTEMTIME st{};
        ULARGE_INTEGER u{};
        u.QuadPart = (ULONGLONG)ms;
        FILETIME ft;
        ft.dwLowDateTime = u.LowPart;
        ft.dwHighDateTime = u.HighPart;
        FileTimeToSystemTime(&ft, &st);

        SYSTEMTIME now{};
        GetLocalTime(&now);
        wchar_t buf[64]{};
        if (st.wYear == now.wYear && st.wMonth == now.wMonth && st.wDay == now.wDay)
            swprintf_s(buf, L"%02d:%02d", st.wHour, st.wMinute);
        else if (st.wYear == now.wYear && st.wDay == now.wDay - 1)
            swprintf_s(buf, L"昨天 %02d:%02d", st.wHour, st.wMinute);
        else
            swprintf_s(buf, L"%d/%d %02d:%02d", st.wMonth, st.wDay, st.wHour, st.wMinute);
        return buf;
    }

    bool ReadFile(const std::wstring& path, std::string& out)
    {
        HANDLE h = CreateFileW(path.c_str(), GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (h == INVALID_HANDLE_VALUE)
            return false;
        DWORD hi = 0;
        DWORD lo = GetFileSize(h, &hi);
        out.resize(lo);
        DWORD rd = 0;
        BOOL ok = ReadFile(h, out.data(), lo, &rd, nullptr);
        CloseHandle(h);
        return ok && rd == lo;
    }

    bool WriteFile(const std::wstring& path, const std::string& data)
    {
        HANDLE h = CreateFileW(path.c_str(), GENERIC_WRITE, 0, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
        if (h == INVALID_HANDLE_VALUE)
            return false;
        DWORD wr = 0;
        BOOL ok = WriteFile(h, data.data(), (DWORD)data.size(), &wr, nullptr);
        CloseHandle(h);
        return ok;
    }

    winrt::Windows::UI::Color ColorFromHex(uint32_t argb)
    {
        winrt::Windows::UI::Color c;
        c.A = (uint8_t)(argb >> 24);
        c.R = (uint8_t)(argb >> 16);
        c.G = (uint8_t)(argb >> 8);
        c.B = (uint8_t)argb;
        return c;
    }

    std::wstring ExeDir()
    {
        wchar_t buf[MAX_PATH]{};
        GetModuleFileNameW(nullptr, buf, MAX_PATH);
        std::wstring p = buf;
        auto pos = p.find_last_of(L'\\');
        return pos == std::wstring::npos ? L"." : p.substr(0, pos);
    }
}
