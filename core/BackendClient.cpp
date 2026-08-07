#include "BackendClient.h"
#include "Util.h"

using namespace winrt::Windows::Data::Json;
using namespace winrt::Windows::Networking::Sockets;
using namespace winrt::Windows::Web::Http;
using namespace winrt::Windows::Web::Http::Filters;

namespace winn
{
    BackendClient::BackendClient(Config& cfg) : m_cfg(cfg)
    {
        m_stop = CreateEventW(nullptr, TRUE, FALSE, nullptr);
    }

    BackendClient::~BackendClient()
    {
        Stop();
        if (m_stop)
            CloseHandle(m_stop);
    }

    void BackendClient::Start()
    {
        if (m_run)
            return;
        m_run = true;
        ResetEvent(m_stop);
        if (m_cfg.backendType == L"poll")
            m_thread = std::thread([this] { PollLoop(); });
        else
            m_thread = std::thread([this] { WsLoop(); });
    }

    void BackendClient::Stop()
    {
        m_run = false;
        SetEvent(m_stop);
        if (m_thread.joinable())
            m_thread.detach();
    }

    void BackendClient::Pause(bool p)
    {
        m_paused = p;
    }

    void BackendClient::WsLoop()
    {
        while (m_run)
        {
            try
            {
                MessageWebSocket ws;
                HANDLE closedEvt = CreateEventW(nullptr, TRUE, FALSE, nullptr);
                ws.Closed([closedEvt](MessageWebSocket const&, WebSocketClosedEventArgs const&) {
                    SetEvent(closedEvt);
                });
                ws.MessageReceived([this](MessageWebSocket const&, MessageWebSocketMessageReceivedEventArgs const& a) {
                    try
                    {
                        auto reader = a.GetDataReader();
                        uint32_t len = reader.UnconsumedBufferLength();
                        std::vector<uint8_t> buf(len);
                        if (len)
                            reader.ReadBytes(buf);
                        HandleRaw(std::string(buf.begin(), buf.end()));
                    }
                    catch (...) {}
                });

                winrt::Windows::Foundation::Uri uri{ winrt::hstring{ m_cfg.backendUrl } };
                ws.ConnectAsync(uri).get();

                HANDLE hs[] = { closedEvt, m_stop };
                DWORD r = WaitForMultipleObjects(2, hs, FALSE, INFINITE);
                CloseHandle(closedEvt);
                if (r == WAIT_OBJECT_0 + 1)
                    break;
            }
            catch (...) {}

            if (!m_run)
                break;
            for (int i = 0; i < 50 && m_run; i++)
                Sleep(100);
        }
    }

    void BackendClient::PollLoop()
    {
        HttpClient hc(HttpBaseProtocolFilter());
        while (m_run)
        {
            bool got = false;
            try
            {
                std::wstring url = m_cfg.backendUrl;
                url += (url.find(L'?') == std::wstring::npos ? L"?" : L"&");
                url += L"since=";
                url += m_since.empty() ? L"0" : m_since;
                winrt::Windows::Foundation::Uri uri{ winrt::hstring{ url } };
                auto resp = hc.GetAsync(uri).get();
                if (resp.StatusCode() == HttpStatusCode::Ok)
                {
                    auto body = resp.Content().ReadAsStringAsync().get();
                    if (!body.empty())
                    {
                        got = true;
                        HandleRaw(winrt::to_string(body));
                    }
                }
            }
            catch (...) {}

            if (!m_run)
                break;
            int delay = got ? 1 : std::max(2, m_cfg.pollIntervalSec);
            for (int i = 0; i < delay * 10 && m_run; i++)
                Sleep(100);
        }
    }

    void BackendClient::HandleRaw(const std::string& utf8)
    {
        auto w = Utf8ToW(utf8);
        try
        {
            auto obj = JsonObject::Parse(winrt::hstring{ w });
            HandleObj(obj);
        }
        catch (...)
        {
            try
            {
                auto arr = JsonArray::Parse(winrt::hstring{ w });
                for (auto&& v : arr)
                {
                    if (v.ValueType() == JsonValueType::Object)
                        HandleObj(v.GetObject());
                }
            }
            catch (...) {}
        }
    }

    void BackendClient::HandleObj(const JsonObject& o)
    {
        if (m_paused)
            return;
        NotifyItem it = NotifyItem::FromJson(o);
        if (!m_cfg.Pass(it))
            return;
        if (!it.id.empty())
            m_since = it.id;
        if (onNotif)
            onNotif(it);
    }
}
