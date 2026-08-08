#include "pch.h"
#include "services/BackendClient.h"
#include "services/ConfigManager.h"

using namespace winrt;
using namespace Windows::Networking::Sockets;
using namespace Windows::Web::Http;
using namespace Windows::Storage::Streams;

BackendClient::BackendClient() = default;

BackendClient::~BackendClient()
{
    Stop();
}

void BackendClient::Start()
{
    if (m_running.exchange(true)) return;

    auto& cfg = ConfigManager::Instance().Get();
    if (cfg.backend.mode == BackendConfig::Mode::WebSocket)
    {
        m_workThread = std::jthread([this] { RunWs(); });
    }
    else
    {
        m_workThread = std::jthread([this] { RunHttpPoll(); });
    }

    if (cfg.backend.autoReconnect)
    {
        m_reconnectThread = std::jthread([this] { ReconnectLoop(); });
    }
}

void BackendClient::Stop()
{
    m_running = false;
    if (m_ws)
    {
        try { m_ws.Close(1000, L"shutdown"); } catch (...) {}
        m_ws = nullptr;
    }
    if (m_workThread.joinable()) m_workThread.join();
    if (m_reconnectThread.joinable()) m_reconnectThread.join();
}

void BackendClient::RunWs()
{
    auto& cfg = ConfigManager::Instance().Get();
    if (cfg.backend.wsUrl.empty()) return;

    while (m_running)
    {
        try
        {
            m_ws = MessageWebSocket();
            m_ws.Control().MessageType(SocketMessageType::Utf8);

            // 消息回调
            m_ws.MessageReceived([this](IWebSocket const&, MessageWebSocketMessageReceivedEventArgs const& args)
            {
                try
                {
                    DataReader reader = args.GetDataReader();
                    reader.UnicodeEncoding(UnicodeEncoding::Utf8);
                    auto len = reader.UnconsumedBufferLength();
                    auto str = reader.ReadString(len);
                    ParseAndDispatch(str.c_str());
                }
                catch (...) {}
            });

            m_ws.Closed([this](IWebSocket const&, WebSocketClosedEventArgs const&)
            {
                m_connected = false;
                if (m_onStatus) m_onStatus(false, L"连接已断开");
            });

            // 设置认证头
            if (!cfg.backend.authToken.empty())
            {
                m_ws.SetRequestHeader(L"Authorization", L"Bearer " + cfg.backend.authToken);
            }

            Uri uri(cfg.backend.wsUrl);
            m_ws.ConnectAsync(uri).get();

            m_connected = true;
            if (m_onStatus) m_onStatus(true, L"已连接");

            // 保持连接，等待断开
            while (m_running && m_connected)
            {
                std::this_thread::sleep_for(std::chrono::seconds(1));
            }
        }
        catch (...)
        {
            m_connected = false;
            if (m_onStatus) m_onStatus(false, L"连接失败");
            if (!m_running) break;
            std::this_thread::sleep_for(std::chrono::milliseconds(cfg.backend.reconnectDelayMs));
        }
    }
}

void BackendClient::RunHttpPoll()
{
    auto& cfg = ConfigManager::Instance().Get();
    if (cfg.backend.httpUrl.empty()) return;

    HttpClient client;

    // 设置认证头
    if (!cfg.backend.authToken.empty())
    {
        client.DefaultRequestHeaders().TryAppendWithoutValidation(
            L"Authorization", L"Bearer " + cfg.backend.authToken);
    }

    m_connected = true;
    if (m_onStatus) m_onStatus(true, L"长轮询中");

    while (m_running)
    {
        try
        {
            Uri uri(cfg.backend.httpUrl);
            auto response = client.GetAsync(uri).get();
            if (response.IsSuccessStatusCode())
            {
                auto body = response.Content().ReadAsStringAsync().get();
                ParseAndDispatch(body.c_str());
            }
        }
        catch (...)
        {
            m_connected = false;
            if (m_onStatus) m_onStatus(false, L"轮询请求失败");
        }

        if (!m_running) break;
        std::this_thread::sleep_for(std::chrono::milliseconds(cfg.backend.pollIntervalMs));
    }

    m_connected = false;
}

void BackendClient::ReconnectLoop()
{
    auto& cfg = ConfigManager::Instance().Get();
    while (m_running)
    {
        std::this_thread::sleep_for(std::chrono::seconds(5));
        if (!m_running) break;

        if (!m_connected && cfg.backend.autoReconnect)
        {
            if (m_onStatus) m_onStatus(false, L"正在重连...");
            // 工作线程会自动重连，这里只做状态通知
        }
    }
}

void BackendClient::ParseAndDispatch(const std::wstring& jsonStr)
{
    try
    {
        JsonObject obj;
        if (!JsonObject::Parse(jsonStr, obj)) return;

        // 支持单条通知
        if (obj.HasKey(L"title"))
        {
            auto n = Notification::FromJson(obj);
            if (m_onNotify) m_onNotify(n);
            return;
        }

        // 支持通知数组
        if (obj.HasKey(L"notifications"))
        {
            auto arr = obj.GetNamedArray(L"notifications");
            for (uint32_t i = 0; i < arr.Size(); i++)
            {
                auto n = Notification::FromJson(arr.GetAt(i).GetObjectW());
                if (m_onNotify) m_onNotify(n);
            }
        }
    }
    catch (...) {}
}

void BackendClient::SendAck(const std::wstring& notifyId)
{
    if (!m_connected || !m_ws) return;

    try
    {
        JsonObject obj;
        obj.SetNamedValue(L"type", JsonValue::CreateStringValue(L"ack"));
        obj.SetNamedValue(L"id", JsonValue::CreateStringValue(notifyId));

        auto writer = DataWriter();
        writer.WriteString(obj.Stringify());
        m_ws.OutputStream().WriteAsync(writer.DetachBuffer());
    }
    catch (...) {}
}
