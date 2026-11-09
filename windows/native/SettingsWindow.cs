using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Media;

namespace Luma;

public sealed class SettingsWindow : Window
{
    public Settings Value { get; private set; }
    public SettingsWindow(Settings current, Action<Settings> saveSettings)
    {
        Value = Core.Copy(current);
        Title = "Luma · Settings"; Width = 560; Height = 700; MinWidth = 450; MinHeight = 400; WindowStartupLocation = WindowStartupLocation.CenterOwner;
        Background = Brush("#20222b"); Foreground = Brush("#edf0f8");
        var panel = new StackPanel { Margin = new Thickness(24) };
        var error = new TextBlock { Tag = "status", Foreground = Brush("#ffb4a9"), TextWrapping = TextWrapping.Wrap, Margin = new Thickness(0, 8, 0, 8) };
        bool restoring = false, closing = false;
        bool Commit(Settings candidate)
        {
            try { candidate.Validate(); saveSettings(candidate); Value = Core.Copy(candidate); ApplyTheme(candidate); error.Text = "Settings saved."; return true; }
            catch (Exception ex) { error.Text = ex.Message; return false; }
        }
        Content = new ScrollViewer { Content = panel, VerticalScrollBarVisibility = ScrollBarVisibility.Auto };
        void Heading(string text, int size = 15) => panel.Children.Add(new TextBlock { Text = text, FontSize = size, FontWeight = FontWeights.Bold, Margin = new Thickness(0, 18, 0, 8), Foreground = Foreground });
        void Info(string text) => panel.Children.Add(new TextBlock { Tag = "info", Text = text, TextWrapping = TextWrapping.Wrap, Foreground = Brush("#b4bfd2"), Margin = new Thickness(0, 3, 0, 8) });
        TextBox Field(string label, string value)
        {
            var row = new Grid { Margin = new Thickness(0, 4, 0, 4) };
            row.ColumnDefinitions.Add(new ColumnDefinition()); row.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(200) });
            row.Children.Add(new TextBlock { Text = label, VerticalAlignment = VerticalAlignment.Center, Foreground = Foreground });
            var input = new TextBox { Text = value, Padding = new Thickness(6), ToolTip = label };
            Grid.SetColumn(input, 1); row.Children.Add(input); panel.Children.Add(row); return input;
        }
        CheckBox Check(string label, bool value)
        {
            var check = new CheckBox { Content = label, IsChecked = value, Foreground = Foreground, Margin = new Thickness(0, 7, 0, 7) };
            panel.Children.Add(check); return check;
        }
        Button Button(string text)
        {
            var button = new Button { Content = text, Padding = new Thickness(10, 7, 10, 7), Margin = new Thickness(0, 5, 0, 5), HorizontalAlignment = HorizontalAlignment.Left, MinWidth = 180 };
            panel.Children.Add(button); return button;
        }
        Heading("Settings", 24);
        Info("Changes save automatically when you leave a field or change an option. Save settings retries any pending edits.");
        Heading("Display and shortcut");
        var themeRow = new Grid { Margin = new Thickness(0, 4, 0, 4) };
        themeRow.ColumnDefinitions.Add(new ColumnDefinition()); themeRow.ColumnDefinitions.Add(new ColumnDefinition { Width = new GridLength(200) });
        themeRow.Children.Add(new TextBlock { Text = "Theme", Foreground = Foreground, VerticalAlignment = VerticalAlignment.Center });
        var theme = new ComboBox { ItemsSource = new[] { "Dark", "Light", "System" }, SelectedItem = Value.Theme, Padding = new Thickness(4) };
        Grid.SetColumn(theme, 1); themeRow.Children.Add(theme); panel.Children.Add(themeRow);
        var width = Field("Window width (360–1200)", Value.Width.ToString());
        var count = Field("Visible results (1–20)", Value.Results.ToString());
        var size = Field("Font size (12–32)", Value.FontSize.ToString());
        var accent = Field("Accent (#RRGGBB)", Value.Accent);
        var shortcut = Field("Toggle launcher shortcut", Value.Hotkey);
        var settingsShortcut = Field("Open Settings shortcut", Value.SettingsHotkey);
        void CaptureShortcut(TextBox field)
        {
            field.IsReadOnly = true; field.ToolTip = "Focus this field and press a modifier plus a key.";
            field.PreviewKeyDown += (sender, e) =>
            {
                var key = e.Key == Key.System ? e.SystemKey : e.Key;
                if (key == Key.Tab || key == Key.Enter && Keyboard.Modifiers == ModifierKeys.None) return;
                string text = HotkeyParser.FromKey(key, Keyboard.Modifiers);
                if (HotkeyParser.TryParse(text, out _, out _)) field.Text = text;
                e.Handled = true;
            };
        }
        CaptureShortcut(shortcut); CaptureShortcut(settingsShortcut);
        Info("Click a shortcut field and press a combination such as Ctrl+Alt+Space. Each shortcut must be different; Windows may reserve some combinations.");
        var compact = Check("Compact layout", Value.Compact);
        var showSettings = Check("Show Settings button on launcher", Value.ShowSettingsButton);
        Heading("Search and history");
        var history = Check("Record launch history", Value.History);
        var descriptions = Check("Show descriptions", Value.Descriptions);
        var fuzzy = Check("Fuzzy matching", Value.Fuzzy);
        var clearOnShow = Check("Clear search when opening", Value.ClearOnShow);
        Heading("Personalization");
        var clear = Button("Clear launch history");
        bool clearArmed = false;
        clear.Click += (_, _) =>
        {
            if (!clearArmed) { clearArmed = true; clear.Content = "Click again to clear history"; return; }
            var candidate = Core.Copy(Value); candidate.Usage.Clear();
            if (Commit(candidate)) { clearArmed = false; clear.Content = "Clear launch history"; }
        };
        var reset = Button("Reset personalization");
        bool resetArmed = false;
        reset.Click += (_, _) =>
        {
            if (!resetArmed) { resetArmed = true; reset.Content = "Click again to reset personalization"; return; }
            var candidate = Core.Copy(Value); candidate.Usage.Clear(); candidate.Favorites.Clear();
            if (Commit(candidate)) { resetArmed = false; reset.Content = "Reset personalization"; }
        };
        Heading("Startup");
        var startup = Check("Start Luma when I sign in", Value.StartAtLogin);
        Info("Windows handles global shortcuts directly. Ctrl+Enter opens Windows application actions. Ranking uses launches recorded by Luma.");
        panel.Children.Add(error);
        var pending = new List<Func<bool>>();
        void AutoField(TextBox field, Func<Settings, string> read, Action<Settings, string> write)
        {
            bool Apply()
            {
                if (field.Text.Trim() == read(Value)) return true;
                try { var candidate = Core.Copy(Value); write(candidate, field.Text.Trim()); return Commit(candidate); }
                catch (Exception ex) { error.Text = ex.Message; return false; }
            }
            pending.Add(Apply);
            field.LostFocus += (_, _) => { if (!closing && IsVisible) Apply(); };
            field.PreviewKeyDown += (_, e) => { if (e.Key == Key.Enter && Keyboard.Modifiers == ModifierKeys.None) { Apply(); e.Handled = true; } };
        }
        AutoField(width, s => s.Width.ToString(), (s, v) => s.Width = int.Parse(v));
        AutoField(count, s => s.Results.ToString(), (s, v) => s.Results = int.Parse(v));
        AutoField(size, s => s.FontSize.ToString(), (s, v) => s.FontSize = int.Parse(v));
        AutoField(accent, s => s.Accent, (s, v) => s.Accent = v);
        AutoField(shortcut, s => s.Hotkey, (s, v) => s.Hotkey = v);
        AutoField(settingsShortcut, s => s.SettingsHotkey, (s, v) => s.SettingsHotkey = v);
        void AutoCheck(CheckBox check, Func<Settings, bool> read, Action<Settings, bool> write)
        {
            check.Click += (_, _) =>
            {
                if (restoring) return;
                var candidate = Core.Copy(Value); write(candidate, check.IsChecked == true);
                if (!Commit(candidate)) { restoring = true; check.IsChecked = read(Value); restoring = false; }
            };
        }
        AutoCheck(compact, s => s.Compact, (s, v) => s.Compact = v);
        AutoCheck(showSettings, s => s.ShowSettingsButton, (s, v) => s.ShowSettingsButton = v);
        AutoCheck(history, s => s.History, (s, v) => s.History = v);
        AutoCheck(descriptions, s => s.Descriptions, (s, v) => s.Descriptions = v);
        AutoCheck(fuzzy, s => s.Fuzzy, (s, v) => s.Fuzzy = v);
        AutoCheck(clearOnShow, s => s.ClearOnShow, (s, v) => s.ClearOnShow = v);
        AutoCheck(startup, s => s.StartAtLogin, (s, v) => s.StartAtLogin = v);
        theme.SelectionChanged += (_, _) =>
        {
            if (restoring) return;
            var candidate = Core.Copy(Value); candidate.Theme = theme.SelectedItem as string ?? "";
            if (!Commit(candidate)) { restoring = true; theme.SelectedItem = Value.Theme; restoring = false; }
        };
        var save = Button("Save settings"); save.IsDefault = true;
        save.Click += (_, _) =>
        {
            if (!int.TryParse(width.Text, out int w) || !int.TryParse(count.Text, out int r) || !int.TryParse(size.Text, out int f)) { error.Text = "Width, result count, and font size must be whole numbers."; return; }
            var candidate = Core.Copy(Value);
            candidate.Width = w; candidate.Results = r; candidate.FontSize = f; candidate.Accent = accent.Text.Trim(); candidate.Theme = theme.SelectedItem as string ?? "";
            candidate.Hotkey = shortcut.Text; candidate.SettingsHotkey = settingsShortcut.Text; candidate.Compact = compact.IsChecked == true; candidate.ShowSettingsButton = showSettings.IsChecked == true;
            candidate.History = history.IsChecked == true; candidate.Descriptions = descriptions.IsChecked == true; candidate.Fuzzy = fuzzy.IsChecked == true;
            candidate.ClearOnShow = clearOnShow.IsChecked == true; candidate.StartAtLogin = startup.IsChecked == true;
            if (Commit(candidate)) { closing = true; DialogResult = true; }
        };
        var cancel = Button("Close");
        Closing += (_, e) => { if (!closing && pending.Any(apply => !apply())) e.Cancel = true; };
        cancel.Click += (_, _) => Close();
        var discard = Button("Discard pending edits"); discard.IsCancel = true;
        discard.Click += (_, _) => { closing = true; DialogResult = false; };
        Info("Discard pending edits closes without retrying unsaved fields. Changes already saved remain in effect.");
        ApplyTheme(Value);
    }
    internal void ApplyTheme(Settings settings)
    {
        bool light = settings.Theme == "Light" || settings.Theme == "System" && MainWindow.IsSystemLight();
        Background = Brush(light ? "#f6f7fb" : "#20222b"); Foreground = Brush(light ? "#202330" : "#edf0f8");
        void Apply(DependencyObject parent)
        {
            foreach (var child in LogicalTreeHelper.GetChildren(parent).OfType<DependencyObject>())
            {
                if (child is TextBlock text && !Equals(text.Tag, "status")) text.Foreground = Equals(text.Tag, "info") ? Brush(light ? "#626a7d" : "#b4bfd2") : Foreground;
                if (child is CheckBox check) check.Foreground = Foreground;
                Apply(child);
            }
        }
        Apply(this);
    }
    private static SolidColorBrush Brush(string color) => new((Color)ColorConverter.ConvertFromString(color));
}
