#include "pch.h"
#include "MainWindow.xaml.h"
#include "App.xaml.h"
#include "core/Config.h"
#include "core/NotifyCenter.h"
#include "core/Util.h"

using namespace winrt;
using namespace winrt::Microsoft::UI::Xaml;
using namespace winrt::Microsoft::UI::Xaml::Controls;
using namespace winrt::Microsoft::UI::Xaml::Media;
using namespace winrt::Microsoft::UI::Xaml::Input;
using namespace winrt::Microsoft::UI::Windowing;
using namespace winrt::Windows::System;

namespace winrt::WinNotify::implementation
{
    static winrt::Windows::UI::Color RowCatColor(winn::Cat c)
    {
        switch (c)
        {
        case winn::Cat::Message: return winn::ColorFromHex(0xFF0A84FF);
        case winn::Cat::Reminder: return winn::ColorFromHex(0xFFFF9F0A);
        case winn::Cat::System: return winn::ColorFromHex(0xFF98989D);
        default: return winn::ColorFromHex(0xFF30D158);
        }
    }

    MainWindow::MainWindow()
    {
        InitializeComponent();

        Title(winrt::hstring{ L"WinNotify" });
        AppWindow().Resize(winrt::Windows::Graphics::SizeInt32{ 430, 660 });

        SearchBox.QueryIcon(SymbolIcon{ Symbol::Find });
        SearchBox.TextChanged([this](auto const&, auto const&) {
            m_query = std::wstring(SearchBox().Text());
            Refresh();
        });

        std::vector<std::pair<std::wstring, int>> cats{
            { L"\u5168\u90E8", 0 }, { L"\u6D88\u606F", 1 }, { L"\u63D0\u9192", 2 },
            { L"\u7CFB\u7EDF", 3 }, { L"\u5176\u4ED6", 4 }
        };
        for (auto& [name, v] : cats)
        {
            auto ci = ComboBoxItem();
            ci.Content(box_value(winrt::hstring{ name }));
            CatBox().Items().Append(ci);
        }
        CatBox().SelectedIndex(0);
        CatBox().SelectionChanged([this](auto const&, auto const&) {
            m_cat = CatBox().SelectedIndex();
            Refresh();
        });

        List().ItemClick([this](auto const&, ItemClickEventArgs const& e) {
            auto li = e.ClickedItem().try_as<ListViewItem>();
            if (!li || !li.Tag())
                return;
            auto id = unbox_value<winrt::hstring>(li.Tag());
            auto& center = App::Instance().Center();
            auto all = center.All();
            for (auto& it : all)
            {
                if (it.id == std::wstring(id))
                {
                    center.MarkRead(it.id);
                    ActivateItem(it);
                    break;
                }
            }
        });

        BtnAllRead().Click([this](auto const&, auto const&) {
            App::Instance().Center().MarkAllRead();
        });
        BtnClear().Click([this](auto const&, auto const&) {
            App::Instance().Center().Clear();
        });

        m_closingToken = AppWindow().Closing([this](auto const&, AppWindowClosingEventArgs const& args) {
            if (App::Instance().Exiting())
                return;
            args.Cancel(true);
            HideWindow();
            App::Instance().Tray().ShowBalloon(L"WinNotify",
                L"\u4ECD\u5728\u540E\u53F0\u8FD0\u884C\uFF0C\u53EF\u4ECE\u6258\u76D8\u56DE\u590D");
        });

        AppWindow().Closed([this](auto const&, WindowEventArgs const&) {
            if (App::Instance().Exiting())
                App::Instance().OnMainClosed();
        });

        Refresh();
    }

    void MainWindow::ShowWindow()
    {
        AppWindow().Show();
        Activate();
    }

    void MainWindow::HideWindow()
    {
        AppWindow().Hide();
    }

    void MainWindow::OnNotif(const winn::NotifyItem& it)
    {
        Refresh();
    }

    void MainWindow::Refresh()
    {
        Rebuild();
        RefreshBadge();
    }

    void MainWindow::RefreshBadge()
    {
        uint32_t u = App::Instance().Center().Unread();
        UnreadText().Text(winrt::hstring{ std::to_wstring(u) });
        UnreadBadge().Visibility(u ? Visibility::Visible : Visibility::Collapsed);
    }

    void MainWindow::Rebuild()
    {
        auto& center = App::Instance().Center();
        auto items = center.All();

        std::vector<winn::NotifyItem> filtered;
        filtered.reserve(items.size());
        for (auto& it : items)
        {
            if (m_cat > 0 && (int)it.cat + 1 != m_cat)
                continue;
            if (!m_query.empty())
            {
                auto q = winn::ToLower(m_query);
                bool m = winn::ToLower(it.title).find(q) != std::wstring::npos
                    || winn::ToLower(it.body).find(q) != std::wstring::npos
                    || winn::ToLower(it.app).find(q) != std::wstring::npos;
                if (!m)
                    continue;
            }
            filtered.push_back(it);
        }

        List().Items().Clear();

        std::array<winn::Cat, 4> order{ winn::Cat::Message, winn::Cat::Reminder, winn::Cat::System, winn::Cat::Other };
        for (auto cat : order)
        {
            std::vector<winn::NotifyItem> grp;
            for (auto& it : filtered)
            {
                if (it.cat == cat)
                    grp.push_back(it);
            }
            if (grp.empty())
                continue;

            auto hb = Border();
            hb.Background(SolidColorBrush{ winn::ColorFromHex(0x0FFFFFFF) });
            hb.CornerRadius(CornerRadius{ 6 });
            hb.Padding(Thickness{ 10, 5, 10, 5 });
            hb.Margin(Thickness{ 0, 10, 0, 4 });
            auto ht = TextBlock();
            ht.Text(winrt::hstring{ winn::NotifyItem::CatName(cat) });
            ht.FontSize(12);
            ht.FontWeight(Microsoft::UI::Text::FontWeights::SemiBold());
            ht.Foreground(SolidColorBrush{ winn::ColorFromHex(0xFFD0D0D4) });
            hb.Child(ht);

            auto hi = ListViewItem();
            hi.Content(hb);
            hi.IsHitTestVisible(false);
            List().Items().Append(hi);

            for (auto& it : grp)
            {
                auto card = Border();
                card.CornerRadius(CornerRadius{ 8 });
                card.Background(SolidColorBrush{ winn::ColorFromHex(0x0AFFFFFF) });
                card.Margin(Thickness{ 0, 2, 0, 2 });
                card.Padding(Thickness{ 12, 8, 12, 8 });

                auto grid = Grid();
                grid.ColumnSpacing(10);
                auto ca = ColumnDefinition();
                ca.Width(GridLengthHelper::Auto());
                auto cb = ColumnDefinition();
                cb.Width(GridLengthHelper::Star());
                grid.ColumnDefinitions().Append(ca);
                grid.ColumnDefinitions().Append(cb);

                auto accent = Border();
                accent.Width(3);
                accent.CornerRadius(CornerRadius{ 2 });
                accent.Background(SolidColorBrush{ RowCatColor(it.cat) });
                accent.VerticalAlignment(VerticalAlignment::Stretch);
                grid.Children().Append(accent);

                auto body = StackPanel();
                body.Spacing(3);
                Grid::SetColumn(body, 1);
                grid.Children().Append(body);

                auto r1 = Grid();
                r1.ColumnSpacing(8);
                auto c1a = ColumnDefinition();
                c1a.Width(GridLengthHelper::Star());
                auto c1b = ColumnDefinition();
                c1b.Width(GridLengthHelper::Auto());
                r1.ColumnDefinitions().Append(c1a);
                r1.ColumnDefinitions().Append(c1b);

                auto title = TextBlock();
                title.Text(winrt::hstring{ it.title });
                title.FontSize(13.5);
                title.TextTrimming(TextTrimming::CharacterEllipsis);
                title.MaxLines(1);
                title.Foreground(SolidColorBrush{ it.read ? winn::ColorFromHex(0xFF9A9AA0) : winn::ColorFromHex(0xFFF2F2F7) });
                title.FontWeight(it.read ? Microsoft::UI::Text::FontWeights::Normal() : Microsoft::UI::Text::FontWeights::SemiBold());
                r1.Children().Append(title);

                auto tm = TextBlock();
                tm.Text(winrt::hstring{ winn::TimeStr(it.time) });
                tm.FontSize(11);
                tm.Foreground(SolidColorBrush{ winn::ColorFromHex(0xFF6E6E73) });
                tm.VerticalAlignment(VerticalAlignment::Center);
                Grid::SetColumn(tm, 1);
                r1.Children().Append(tm);
                body.Children().Append(r1);

                auto msg = TextBlock();
                msg.Text(winrt::hstring{ it.body });
                msg.FontSize(12);
                msg.Foreground(SolidColorBrush{ winn::ColorFromHex(0xFFA8A8AE) });
                msg.TextWrapping(TextWrapping::Wrap);
                msg.TextTrimming(TextTrimming::CharacterEllipsis);
                msg.MaxLines(2);
                body.Children().Append(msg);

                auto src = TextBlock();
                std::wstring s = it.app.empty() ? winn::NotifyItem::CatName(it.cat) : it.app;
                if (it.pri == winn::Pri::High)
                    s += L" \u00B7 \u9AD8\u4F18\u5148";
                src.Text(winrt::hstring{ s });
                src.FontSize(11);
                src.Foreground(SolidColorBrush{ winn::ColorFromHex(0xFF6E6E73) });
                body.Children().Append(src);

                card.Child(grid);

                auto li = ListViewItem();
                li.Content(card);
                li.Padding(Thickness{ 0, 2, 0, 2 });
                li.Tag(box_value(winrt::hstring{ it.id }));
                card.RightTapped([this, it, card](auto const&, RightTappedRoutedEventArgs const&) {
                    ShowItemMenu(it, card);
                });
                List().Items().Append(li);
            }
        }
    }

    void MainWindow::ShowItemMenu(const winn::NotifyItem& it, FrameworkElement anchor)
    {
        auto menu = MenuFlyout();

        if (!it.actionUrl.empty())
        {
            auto open = MenuFlyoutItem();
            open.Text(winrt::hstring{ L"\u6253\u5F00\u94FE\u63A5" });
            open.Icon(SymbolIcon{ Symbol::Link });
            open.Click([this, it](auto const&, auto const&) { ActivateItem(it); });
            menu.Items().Append(open);
        }

        if (!it.read)
        {
            auto read = MenuFlyoutItem();
            read.Text(winrt::hstring{ L"\u6807\u8BB0\u5DF2\u8BFB" });
            read.Click([this, it](auto const&, auto const&) {
                App::Instance().Center().MarkRead(it.id);
            });
            menu.Items().Append(read);
        }

        auto del = MenuFlyoutItem();
        del.Text(winrt::hstring{ L"\u5220\u9664" });
        del.Icon(SymbolIcon{ Symbol::Delete });
        del.Click([this, it](auto const&, auto const&) {
            App::Instance().Center().Remove(it.id);
        });
        menu.Items().Append(del);

        if (menu.Items().Size() > 0)
            menu.ShowAt(anchor);
    }

    void MainWindow::ActivateItem(const winn::NotifyItem& it)
    {
        App::Instance().HandleAction(it);
    }
}
