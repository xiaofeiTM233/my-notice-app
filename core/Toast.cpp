#include "Toast.h"
#include "Util.h"

#include <dwmapi.h>
#include <microsoft.ui.xaml.window.h>

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace winrt::Microsoft::UI::Xaml::Media;
using namespace winrt::Microsoft::UI::Xaml::Hosting;
using namespace winrt::Microsoft::UI::Xaml::Input;
using namespace winrt::Microsoft::UI::Windowing;
using namespace winrt::Windows::Foundation::Numerics;

namespace winn
{
    static winrt::Windows::UI::Color CatColor(Cat c)
    {
        switch (c)
        {
        case Cat::Message: return ColorFromHex(0xFF0A84FF);
        case Cat::Reminder: return ColorFromHex(0xFFFF9F0A);
        case Cat::System: return ColorFromHex(0xFF98989D);
        default: return ColorFromHex(0xFF30D158);
        }
    }

    Toast::Toast(const Config& cfg, const NotifyItem& it, std::function<void(const NotifyItem&)> onAct)
        : m_cfg(cfg), m_item(it), m_onAct(std::move(onAct))
    {
        BuildUi();
    }

    Toast::~Toast()
    {
        if (m_win)
            m_win.Close();
    }

    void Toast::BuildUi()
    {
        m_win = Window();

        auto appWin = m_win.AppWindow();
        appWin.Title(winrt::hstring{ m_item.app.empty() ? L"WinNotify" : m_item.app });
        auto presenter = appWin.Presenter().as<OverlappedPresenter>();
        presenter.SetBorderAndTitleBar(false);
        presenter.IsResizable(false);
        presenter.IsMaximizable(false);
        presenter.IsMinimizable(false);
        presenter.IsAlwaysOnTop(true);
        appWin.IsShownInSwitchers(false);

        m_win.SystemBackdrop(nullptr);

        auto native = m_win.as<::IWindowNative>();
        HWND hwnd = nullptr;
        winrt::check_hresult(native->get_WindowHandle(&hwnd));
        MARGINS mg{ -1, -1, -1, -1 };
        DwmExtendFrameIntoClientArea(hwnd, &mg);

        auto card = Border();
        card.CornerRadius(CornerRadius{ 12 });
        card.Background(SolidColorBrush{ ColorFromHex(0xF21B1D23) });
        card.BorderThickness(Thickness{ 1 });
        card.BorderBrush(SolidColorBrush{ ColorFromHex(0x26FFFFFF) });

        auto grid = Grid();
        grid.Margin(Thickness{ 16, 10, 8, 12 });
        grid.RowSpacing(4);

        auto rd0 = RowDefinition();
        rd0.Height(GridLengthHelper::Auto());
        auto rd1 = RowDefinition();
        rd1.Height(GridLengthHelper::Auto());
        auto rd2 = RowDefinition();
        rd2.Height(GridLengthHelper::Auto());
        grid.RowDefinitions().Append(rd0);
        grid.RowDefinitions().Append(rd1);
        grid.RowDefinitions().Append(rd2);

        auto header = Grid();
        header.ColumnSpacing(6);
        auto cd0 = ColumnDefinition();
        cd0.Width(GridLengthHelper::Auto());
        auto cd1 = ColumnDefinition();
        cd1.Width(GridLengthHelper::Auto());
        auto cd2 = ColumnDefinition();
        cd2.Width(GridLengthHelper::Star());
        auto cd3 = ColumnDefinition();
        cd3.Width(GridLengthHelper::Auto());
        auto cd4 = ColumnDefinition();
        cd4.Width(GridLengthHelper::Auto());
        header.ColumnDefinitions().Append(cd0);
        header.ColumnDefinitions().Append(cd1);
        header.ColumnDefinitions().Append(cd2);
        header.ColumnDefinitions().Append(cd3);
        header.ColumnDefinitions().Append(cd4);

        auto dot = Ellipse();
        dot.Width(8);
        dot.Height(8);
        dot.Fill(SolidColorBrush{ CatColor(m_item.cat) });
        dot.VerticalAlignment(VerticalAlignment::Center);
        header.Children().Append(dot);

        auto cat = TextBlock();
        cat.Text(winrt::hstring{ NotifyItem::CatName(m_item.cat) });
        cat.Foreground(SolidColorBrush{ CatColor(m_item.cat) });
        cat.FontSize(11);
        cat.VerticalAlignment(VerticalAlignment::Center);
        Grid::SetColumn(cat, 1);
        header.Children().Append(cat);

        auto time = TextBlock();
        time.Text(winrt::hstring{ TimeStr(m_item.time) });
        time.Foreground(SolidColorBrush{ ColorFromHex(0xFF8E8E93) });
        time.FontSize(11);
        time.VerticalAlignment(VerticalAlignment::Center);
        time.HorizontalAlignment(HorizontalAlignment::Right);
        Grid::SetColumn(time, 3);
        header.Children().Append(time);

        auto close = Button();
        close.Width(26);
        close.Height(26);
        close.Padding(Thickness{ 0 });
        close.Background(SolidColorBrush{ ColorFromHex(0x00000000) });
        close.BorderThickness(Thickness{ 0 });
        close.HorizontalAlignment(HorizontalAlignment::Right);
        close.VerticalAlignment(VerticalAlignment::Center);
        auto icon = FontIcon();
        icon.Glyph(winrt::hstring{ L"\uE8BB" });
        icon.FontSize(11);
        close.Content(icon);
        close.Click([this](auto, auto) { Close(); });
        Grid::SetColumn(close, 4);
        header.Children().Append(close);

        if (m_item.pri == Pri::High)
        {
            auto hi = TextBlock();
            hi.Text(winrt::hstring{ L"\u00B7 \u9AD8\u4F18\u5148" });
            hi.Foreground(SolidColorBrush{ ColorFromHex(0xFFFF453A) });
            hi.FontSize(11);
            hi.VerticalAlignment(VerticalAlignment::Center);
            Grid::SetColumn(hi, 2);
            header.Children().Append(hi);
        }

        auto title = TextBlock();
        title.Text(winrt::hstring{ m_item.title });
        title.FontSize(14);
        title.FontWeight(Microsoft::UI::Text::FontWeights::SemiBold());
        title.Foreground(SolidColorBrush{ ColorFromHex(0xFFF2F2F7) });
        title.TextTrimming(TextTrimming::CharacterEllipsis);
        title.MaxLines(1);
        Grid::SetRow(title, 1);
        grid.Children().Append(title);

        auto body = TextBlock();
        body.Text(winrt::hstring{ m_item.body });
        body.FontSize(12.5);
        body.Foreground(SolidColorBrush{ ColorFromHex(0xFFB8B8BC) });
        body.TextWrapping(TextWrapping::Wrap);
        body.TextTrimming(TextTrimming::CharacterEllipsis);
        body.MaxLines(2);
        Grid::SetRow(body, 2);
        grid.Children().Append(body);

        card.Child(grid);
        m_win.Content(card);

        card.Tapped([this, card](auto const&, TappedRoutedEventArgs const&) {
            Close();
            if (m_onAct)
                m_onAct(m_item);
        });

        card.Loaded([this, card](auto const&, auto const&) {
            AddShadow();
            SlideIn();
            Layout(m_index);
            m_visible = true;
        });

        m_timer = DispatcherTimer();
        m_timer.Interval(std::chrono::milliseconds{ (int64_t)(m_cfg.popupDurationSec * 1000) });
        m_timer.Tick([this](auto, auto) { Close(); });
        m_timer.Start();
    }

    void Toast::AddShadow()
    {
        auto card = m_win.Content().as<Border>();
        auto v = ElementCompositionPreview::GetElementVisual(card);
        auto compositor = v.Compositor();
        auto shadow = compositor.CreateDropShadow();
        shadow.BlurRadius(28.f);
        shadow.Opacity(0.45f);
        shadow.Color(winrt::Windows::UI::Colors::Black());
        auto sprite = compositor.CreateSpriteVisual();
        sprite.Shadow(shadow);
        ElementCompositionPreview::SetElementChildVisual(card, sprite);
        card.SizeChanged([sprite](auto const& sender, auto const&) {
            sprite.Size(sender.ActualSize());
        });
    }

    void Toast::SlideIn()
    {
        auto card = m_win.Content().as<Border>();
        ElementCompositionPreview::SetIsTranslationEnabled(card, true);
        auto v = ElementCompositionPreview::GetElementVisual(card);
        auto compositor = v.Compositor();

        auto slide = compositor.CreateVector3KeyFrameAnimation();
        slide.InsertKeyFrame(0.f, float3{ 90.f, 0.f, 0.f });
        slide.InsertKeyFrame(1.f, float3{ 0.f, 0.f, 0.f });
        slide.Duration(std::chrono::milliseconds{ 320 });
        v.StartAnimation(L"Translation", slide);

        auto fade = compositor.CreateScalarKeyFrameAnimation();
        fade.InsertKeyFrame(0.f, 0.f);
        fade.InsertKeyFrame(1.f, 1.f);
        fade.Duration(std::chrono::milliseconds{ 240 });
        v.StartAnimation(L"Opacity", fade);
    }

    void Toast::Layout(int index)
    {
        if (!m_win.Content())
            return;
        double scale = m_win.Content().XamlRoot().RasterizationScale();
        if (scale <= 0.1)
            scale = 1.0;

        RECT wa{};
        SystemParametersInfoW(SPI_GETWORKAREA, 0, &wa, 0);

        int w = (int)(m_cfg.popupWidth * scale);
        int h = (int)(132 * scale);
        int gap = (int)(10 * scale);
        int x = wa.right - w - (int)(16 * scale);
        int y = wa.bottom - h - (int)(16 * scale) - index * (h + gap);

        m_win.AppWindow().MoveAndResize(winrt::Windows::Graphics::RectInt32{ x, y, w, h });
    }

    void Toast::Show()
    {
        m_win.AppWindow().Show();
    }

    void Toast::Close()
    {
        if (!m_visible)
            return;
        m_visible = false;
        m_timer.Stop();
        m_win.AppWindow().Hide();
        if (onClosed)
            onClosed(this);
    }

    ToastMgr::ToastMgr(const Config& cfg) : m_cfg(cfg)
    {
    }

    void ToastMgr::Push(const NotifyItem& it, std::function<void(const NotifyItem&)> onAct)
    {
        while ((int)m_toasts.size() >= m_cfg.popupMaxStack)
        {
            m_toasts.front()->Close();
        }
        auto t = std::make_shared<Toast>(m_cfg, it, onAct);
        t->onClosed = [this](Toast* p) { Remove(p); };
        m_toasts.push_back(t);
        Relayout();
        t->Show();
    }

    void ToastMgr::Remove(Toast* t)
    {
        auto dup = std::find_if(m_toasts.begin(), m_toasts.end(),
            [t](const std::shared_ptr<Toast>& x) { return x.get() == t; });
        if (dup != m_toasts.end())
            m_toasts.erase(dup);
        Relayout();
    }

    void ToastMgr::CloseAll()
    {
        auto list = m_toasts;
        m_toasts.clear();
        for (auto& t : list)
            t->Close();
    }

    void ToastMgr::Relayout()
    {
        for (size_t i = 0; i < m_toasts.size(); i++)
        {
            m_toasts[i]->SetIndex((int)i);
            m_toasts[i]->Layout((int)i);
        }
    }
}
