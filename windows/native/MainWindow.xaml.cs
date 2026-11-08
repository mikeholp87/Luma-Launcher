using System.Diagnostics;
using System.IO;
using System.Text.Json;
using System.Windows;
using System.Windows.Controls;
using System.Windows.Input;
using System.Windows.Interop;
using System.Windows.Media;
using Microsoft.Win32;
using System.Windows.Threading;

namespace Luma;

public partial class MainWindow : Window
{
    private Settings settings;
    private bool writable = true;
    private string? warning;
    private Entry[] entries = [];
    private readonly Dictionary<string, Task<ImageSource?>> icons = new(StringComparer.OrdinalIgnoreCase);
    private readonly SemaphoreSlim iconWorkers = new(4);
    private IntPtr hwnd;
    private ShortcutBindings? shortcuts;
    private SettingsWindow? settingsDialog;
    private Window? actionsDialog;
    private readonly List<FileSystemWatcher> indexWatchers = [];
    private readonly DispatcherTimer refreshTimer = new() { Interval = TimeSpan.FromMilliseconds(500) };
    private bool indexing, refreshPending, disposed;
    private readonly PalettePosition position;

    public MainWindow()
    {
        try { settings = Core.Load(Core.DataPath); }
        catch (Exception e) { settings = new Settings(); writable = false; warning = "Settings were preserved and cannot be saved: " + e.Message; }
        InitializeComponent();
        position = new PalettePosition(this);
        ApplyAppearance();
        Deactivated += (_, _) => { if (settingsDialog is null && actionsDialog is null) Hide(); };
        SystemEvents.UserPreferenceChanged += SystemThemeChanged;
        refreshTimer.Tick += (_, _) => { refreshTimer.Stop(); RefreshIndex(); };
        WatchApplications();
        RefreshIndex();
    }
    public void InitializeResident()
    {
        hwnd = new WindowInteropHelper(this).EnsureHandle();
        HwndSource.FromHwnd(hwnd).AddHook(WndProc);
        shortcuts = new ShortcutBindings(TryRegister, id => Native.UnregisterHotKey(hwnd, id));
        shortcuts.Initialize(settings.Hotkey, settings.SettingsHotkey);
        if (!shortcuts.Current.LauncherActive || !shortcuts.Current.SettingsActive)
            warning = string.Join(" ", new[] { warning, !shortcuts.Current.LauncherActive ? "Launcher shortcut unavailable." : null, !shortcuts.Current.SettingsActive ? "Settings shortcut unavailable." : null, "Open Settings to choose another." }.Where(s => !string.IsNullOrEmpty(s)));
        Render();
    }
    private bool TryRegister(string shortcut, int id) => HotkeyParser.TryParse(shortcut, out uint modifiers, out uint key) && Native.RegisterHotKey(hwnd, id, modifiers | 0x4000, key);
    private IntPtr WndProc(IntPtr handle, int msg, IntPtr wParam, IntPtr lParam, ref bool handled)
    {
        if (msg is 0x007E or 0x001A or 0x02E0) position.Schedule(); // Display, work area, or DPI changed.
        if (msg == 0x0312 && shortcuts is not null)
        {
            int id = wParam.ToInt32();
            if (id == 1 && shortcuts.Current.LauncherActive) { if (settingsDialog is null && actionsDialog is null) Toggle(); handled = true; }
            else if (id == 2 && shortcuts.Current.SettingsActive)
            {
                if (settingsDialog is null) { if (!IsVisible) ShowPalette(); else Activate(); OpenSettings(); }
                else settingsDialog.Activate();
                handled = true;
            }
        }
        return IntPtr.Zero;
    }
    public void Toggle() { if (IsVisible) Hide(); else ShowPalette(); }
    public void ShowPalette()
    {
        if (settingsDialog is not null) { settingsDialog.Activate(); return; }
        position.BeginShow();
        ApplyAppearance(); Render();
        if (settings.ClearOnShow) Query.Text = "";
        WindowState = WindowState.Normal;
        Show(); UpdateLayout(); position.Center(); Activate(); Query.Focus(); Query.SelectAll();
    }
    private async void RefreshIndex()
    {
        if (indexing) { refreshPending = true; return; }
        indexing = true;
        icons.Clear();
        try { entries = await Task.Run(() => Core.Discover().ToArray()); }
        catch (Exception e) { warning = "Could not refresh applications: " + e.Message; }
        if (disposed) return;
        indexing = false;
        Render();
        if (refreshPending) { refreshPending = false; RefreshIndex(); }
    }
    private void WatchApplications()
    {
        // Watch the parent when Programs does not exist yet, including folder renames.
        foreach (string root in new[] { Environment.GetFolderPath(Environment.SpecialFolder.Programs), Environment.GetFolderPath(Environment.SpecialFolder.CommonPrograms) }.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            if (string.IsNullOrEmpty(root)) continue;
            string? watchRoot = Directory.Exists(root) ? root : Path.GetDirectoryName(root);
            if (watchRoot is null || !Directory.Exists(watchRoot)) continue;
            try
            {
                var watcher = new FileSystemWatcher(watchRoot) { IncludeSubdirectories = true, NotifyFilter = NotifyFilters.FileName | NotifyFilters.DirectoryName | NotifyFilters.LastWrite };
                watcher.Created += IndexChanged; watcher.Deleted += IndexChanged; watcher.Changed += IndexChanged; watcher.Renamed += IndexChanged;
                watcher.Error += (_, _) => ScheduleRefresh();
                indexWatchers.Add(watcher); watcher.EnableRaisingEvents = true;
            }
            catch (IOException) { }
            catch (UnauthorizedAccessException) { }
        }
    }
    private void IndexChanged(object sender, FileSystemEventArgs e) => ScheduleRefresh();
    private void ScheduleRefresh()
    {
        if (disposed || Dispatcher.HasShutdownStarted) return;
        Dispatcher.BeginInvoke(new Action(() => { if (!disposed) { refreshTimer.Stop(); refreshTimer.Start(); } }));
    }
    private void SystemThemeChanged(object sender, UserPreferenceChangedEventArgs e)
    {
        if (!disposed && !Dispatcher.HasShutdownStarted) Dispatcher.BeginInvoke(new Action(() => { if (!disposed && settings.Theme == "System") { ApplyAppearance(); settingsDialog?.ApplyTheme(settings); } }));
    }
    internal void Command(string command)
    {
        switch (command)
        {
            case "--hide": if (settingsDialog is null && actionsDialog is null) Hide(); break;
            case "--toggle": if (settingsDialog is not null) settingsDialog.Activate(); else Toggle(); break;
            case "--settings": if (!IsVisible) ShowPalette(); OpenSettings(); break;
            case "--refresh": RefreshIndex(); break;
            case "--quit": Application.Current.Shutdown(); break;
            case "--background": break;
            default: ShowPalette(); break;
        }
    }
    private void QueryChanged(object sender, TextChangedEventArgs e) { if (Results != null) Render(); }
    private void Render()
    {
        if (Results == null || Query == null) return;
        string text = Query.Text.Trim();
        Placeholder.Visibility = text.Length == 0 ? Visibility.Visible : Visibility.Collapsed;
        var results = new List<Result>();
        string? note = warning;
        if (text.StartsWith('='))
        {
            try { string value = Core.Calculate(text[1..]); results.Add(new Result(value, "Press Enter to copy result", "calculation", value)); }
            catch { note = "Enter a valid arithmetic expression"; }
        }
        else if (Core.WebUrl(text) is string url) results.Add(new Result("Open web search or URL", url, "url", url));
        else foreach (var app in Core.Search(entries, text, settings))
        {
            bool favorite = settings.Favorites.Contains(app.Path);
            var item = new Result((favorite ? "★  " : "") + app.Name, settings.Descriptions ? app.Description : "", "app", app.Path, favorite);
            results.Add(item);
            LoadIcon(item);
        }
        Results.ItemsSource = results;
        Results.MaxHeight = Math.Min(720, Math.Max(1, results.Count) * (Scaled(settings.FontSize, 36, 24) + (settings.Compact ? 12 : 20)));
        if (results.Count == 0) note ??= indexing ? "Loading installed applications…" : "No matching applications";
        if (results.Count > 0) Results.SelectedIndex = 0;
        Status.Text = note ?? "↑↓ Navigate    Enter Open    Esc Close    Ctrl+D Pin";
    }
    private async void LoadIcon(Result result)
    {
        if (!icons.TryGetValue(result.Value, out var icon))
        {
            icon = ExtractIcon(result.Value);
            icons[result.Value] = icon;
        }
        result.Icon = await icon;
    }
    private async Task<ImageSource?> ExtractIcon(string path)
    {
        await iconWorkers.WaitAsync();
        try { return await Task.Run(() => ShellIcons.Load(path)); }
        finally { iconWorkers.Release(); }
    }
    private void OpenSelected()
    {
        if (Results.SelectedItem is not Result result) return;
        try
        {
            if (result.Kind == "calculation") Clipboard.SetText(result.Value);
            else
            {
                Process.Start(new ProcessStartInfo(result.Value) { UseShellExecute = true });
                if (result.Kind == "app" && settings.History && writable)
                {
                    var usage = settings.Usage.GetValueOrDefault(result.Value) ?? new Usage();
                    usage.Count = usage.Count == long.MaxValue ? long.MaxValue : usage.Count + 1;
                    usage.Last = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
                    settings.Usage[result.Value] = usage;
                    while (settings.Usage.Count > 1000) settings.Usage.Remove(settings.Usage.MinBy(x => x.Value.Last).Key);
                    Persist();
                }
            }
            Hide();
        }
        catch (Exception e) { Status.Text = "Could not open result: " + e.Message; }
    }
    private bool Persist()
    {
        if (!writable) { Status.Text = warning; return false; }
        try { Core.Save(Core.DataPath, settings); return true; }
        catch (Exception e) { warning = "Could not save settings: " + e.Message; Status.Text = warning; return false; }
    }
    private void ApplyAppearance()
    {
        bool light = settings.Theme == "Light" || (settings.Theme == "System" && IsSystemLight());
        void Brush(string name, string color) => Resources[name] = new SolidColorBrush((Color)ColorConverter.ConvertFromString(color));
        Brush("CanvasBrush", light ? "#f6f7fb" : "#20222b"); Brush("SurfaceBrush", light ? "#ffffff" : "#282b36");
        Brush("TextBrush", light ? "#202330" : "#edf0f8"); Brush("MutedBrush", light ? "#626a7d" : "#9ba4b8");
        Brush("BorderBrush", light ? "#d5dbea" : "#4a5062"); Brush("SelectedBrush", light ? "#e1e7f8" : "#333c58"); Brush("AccentBrush", settings.Accent);
        Width = settings.Width; Query.FontSize = settings.FontSize; Placeholder.FontSize = settings.FontSize;
        Results.FontSize = Scaled(settings.FontSize, 15, 11); Status.FontSize = Scaled(settings.FontSize, 12, 10);
        Resources["SmallFontSize"] = (double)Scaled(settings.FontSize, 12, 10);
        Resources["IconSize"] = (double)Scaled(settings.FontSize, 36, 24);
        SettingsButton.Visibility = settings.ShowSettingsButton ? Visibility.Visible : Visibility.Collapsed;
        Results.ItemContainerStyle = new Style(typeof(ListBoxItem), (Style)FindResource(typeof(ListBoxItem))) { Setters = { new Setter(PaddingProperty, new Thickness(12, settings.Compact ? 6 : 10, 12, settings.Compact ? 6 : 10)) } };
    }
    internal static int Scaled(int fontSize, int pixels, int minimum) => Math.Max(minimum, (int)Math.Round(fontSize * pixels / 19.0));
    internal static bool IsSystemLight()
    {
        using var key = Registry.CurrentUser.OpenSubKey(@"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
        return (int)(key?.GetValue("AppsUseLightTheme", 0) ?? 0) != 0;
    }
    private void OnKeyDown(object sender, KeyEventArgs e)
    {
        var (action, index) = KeyboardCommands.Resolve(e.Key, Keyboard.Modifiers.HasFlag(ModifierKeys.Control));
        if (action == PaletteAction.Settings) { OpenSettings(); e.Handled = true; return; }
        if (action == PaletteAction.Actions) { OpenActions(); e.Handled = true; return; }
        if (action == PaletteAction.Jump)
        {
            if (index < Results.Items.Count && Results.Items[index] is Result { Kind: "app" }) { Results.SelectedIndex = index; OpenSelected(); }
            e.Handled = true; return;
        }
        if (action is PaletteAction.Favorite or PaletteAction.Forget && Results.SelectedItem is Result { Kind: "app" } app && writable)
        {
            var next = Core.Copy(settings);
            if (action == PaletteAction.Favorite) { if (!next.Favorites.Remove(app.Value)) next.Favorites.Add(app.Value); }
            else next.Usage.Remove(app.Value);
            try { Core.Save(Core.DataPath, next); settings = next; }
            catch (Exception ex) { warning = "Could not save personalization: " + ex.Message; }
            Render(); e.Handled = true; return;
        }
        if (action == PaletteAction.Escape) { if (Query.Text.Length > 0) Query.Clear(); else Hide(); e.Handled = true; }
        else if (action == PaletteAction.Open) { OpenSelected(); e.Handled = true; }
        else if (action == PaletteAction.Refresh) { RefreshIndex(); e.Handled = true; }
        else if (action == PaletteAction.Next) { Navigate(1); e.Handled = true; }
        else if (action == PaletteAction.Previous) { Navigate(-1); e.Handled = true; }
    }
    private void Navigate(int delta)
    {
        if (Results.Items.Count == 0) return;
        Results.SelectedIndex = Math.Clamp(Results.SelectedIndex + delta, 0, Results.Items.Count - 1);
        Results.ScrollIntoView(Results.SelectedItem);
    }
    private void OpenActions()
    {
        if (Results.SelectedItem is not Result { Kind: "app" } result) return;
        var panel = new StackPanel { Margin = new Thickness(20) };
        var dialog = new Window { Title = result.Name + " · Application actions", Owner = this, Content = panel, Width = 360, SizeToContent = SizeToContent.Height, WindowStartupLocation = WindowStartupLocation.CenterOwner };
        void ActionButton(string label, Action action)
        {
            var button = new Button { Content = label, Padding = new Thickness(12, 8, 12, 8), Margin = new Thickness(0, 4, 0, 4) };
            button.Click += (_, _) => { try { action(); dialog.Close(); Hide(); } catch (Exception ex) { MessageBox.Show(dialog, ex.Message, "Could not open action"); } };
            panel.Children.Add(button);
        }
        ActionButton("Open", OpenSelected);
        foreach (string verb in new ProcessStartInfo(result.Value).Verbs.Where(v => !v.Equals("open", StringComparison.OrdinalIgnoreCase)))
            ActionButton(verb.Equals("runas", StringComparison.OrdinalIgnoreCase) ? "Run as administrator" : verb, () => Process.Start(new ProcessStartInfo(result.Value) { UseShellExecute = true, Verb = verb }));
        ActionButton("Open shortcut location", () => Process.Start(new ProcessStartInfo("explorer.exe") { ArgumentList = { "/select,", result.Value }, UseShellExecute = true }));
        dialog.PreviewKeyDown += (_, e) => { if (e.Key == Key.Escape) { dialog.Close(); e.Handled = true; } };
        actionsDialog = dialog;
        try { dialog.ShowDialog(); } finally { actionsDialog = null; }
    }
    private void SettingsClicked(object sender, RoutedEventArgs e) => OpenSettings();
    private void QuitClicked(object sender, RoutedEventArgs e) => Application.Current.Shutdown();
    private void ResultDoubleClick(object sender, MouseButtonEventArgs e) => OpenSelected();
    private void ResultClicked(object sender, MouseButtonEventArgs e)
    {
        if (e.ChangedButton == MouseButton.Left && Results.SelectedItem is Result) OpenSelected();
    }
    protected override void OnClosed(EventArgs e)
    {
        disposed = true; refreshTimer.Stop();
        position.Dispose();
        foreach (var watcher in indexWatchers) watcher.Dispose();
        SystemEvents.UserPreferenceChanged -= SystemThemeChanged;
        if (shortcuts?.Current.LauncherActive == true) Native.UnregisterHotKey(hwnd, 1);
        if (shortcuts?.Current.SettingsActive == true) Native.UnregisterHotKey(hwnd, 2);
        base.OnClosed(e); Application.Current.Shutdown();
    }

    private void OpenSettings()
    {
        if (settingsDialog is not null) { settingsDialog.Activate(); return; }
        var dialog = new SettingsWindow(settings, SaveSettings) { Owner = this };
        settingsDialog = dialog;
        try { dialog.ShowDialog(); }
        finally { settingsDialog = null; }
    }
    private void SaveSettings(Settings next)
    {
        if (!writable) throw new IOException(warning);
        if (shortcuts is null) throw new InvalidOperationException("Shortcuts have not been initialized. Try opening Settings again.");
        ShortcutState previous = shortcuts.Current;
        bool changed = !string.Equals(next.Hotkey, settings.Hotkey, StringComparison.OrdinalIgnoreCase) || !string.Equals(next.SettingsHotkey, settings.SettingsHotkey, StringComparison.OrdinalIgnoreCase) || !previous.LauncherActive || !previous.SettingsActive;
        if (changed && !shortcuts.TryApply(next.Hotkey, next.SettingsHotkey, out bool restored))
        {
            string detail = restored ? "Previously active shortcuts were restored." : "Some previous shortcuts could not be restored; open Settings to try again.";
            if (!restored) { warning = detail; Render(); }
            throw new InvalidOperationException("One of those shortcuts is unavailable. " + detail);
        }
        try
        {
            // Save before changing startup; roll back settings on startup failure.
            Core.Save(Core.DataPath, next);
            try { if (next.StartAtLogin || next.StartAtLogin != settings.StartAtLogin) SetStartup(next.StartAtLogin); }
            catch { Core.Save(Core.DataPath, settings); throw; }
        }
        catch (Exception ex)
        {
            bool rollbackSucceeded = !changed || shortcuts.Restore(previous);
            string detail = rollbackSucceeded ? "" : " Some previous shortcuts could not be restored; open Settings to try again.";
            if (!rollbackSucceeded) { warning = detail.Trim(); Render(); }
            throw new IOException("Could not save settings: " + ex.Message + detail, ex);
        }
        settings = next; warning = null; ApplyAppearance(); Render();
    }
    private static void SetStartup(bool enabled)
    {
        using var key = Registry.CurrentUser.CreateSubKey(@"Software\Microsoft\Windows\CurrentVersion\Run", true) ?? throw new IOException("Cannot open startup settings.");
        if (enabled)
        {
            string exe = Environment.ProcessPath ?? throw new IOException("Cannot determine the application path.");
            key.SetValue("LumaLauncher", $"\"{exe}\" --background");
        }
        else key.DeleteValue("LumaLauncher", false);
    }
}
