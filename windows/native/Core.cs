using System.Data;
using System.Globalization;
using System.IO;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.ComponentModel;
using System.Windows.Media;
using System.Text;

namespace Luma;

public sealed class Settings
{
    public string Theme { get; set; } = "Dark";
    public string Accent { get; set; } = "#8da9ff";
    public int Width { get; set; } = 680;
    public int Results { get; set; } = 8;
    public int FontSize { get; set; } = 19;
    public bool Compact { get; set; }
    public bool ShowSettingsButton { get; set; } = true;
    public bool History { get; set; } = true;
    public bool Descriptions { get; set; } = true;
    public bool Fuzzy { get; set; } = true;
    public bool ClearOnShow { get; set; } = true;
    public bool StartAtLogin { get; set; }
    public string Hotkey { get; set; } = "Ctrl+Alt+Space";
    public string SettingsHotkey { get; set; } = "Ctrl+Alt+S";
    public Dictionary<string, string> SearchEngines { get; set; } = Core.DefaultEngines();
    public HashSet<string> Favorites { get; set; } = new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, Usage> Usage { get; set; } = new(StringComparer.OrdinalIgnoreCase);
    public void Validate()
    {
        if (Theme is not ("Dark" or "Light" or "System") || !Regex.IsMatch(Accent ?? "", "^#[0-9a-fA-F]{6}$") || Width is < 360 or > 1200 || Results is < 1 or > 20 || FontSize is < 12 or > 32 || Favorites is null || Usage is null || !HotkeyParser.TryParse(Hotkey, out uint launcherModifiers, out uint launcherKey) || !HotkeyParser.TryParse(SettingsHotkey, out uint settingsModifiers, out uint settingsKey))
            throw new InvalidDataException("Invalid settings. Check theme, accent, sizes, and shortcut.");
        if (SearchEngines is null || SearchEngines.Count > 32) throw new InvalidDataException("At most 32 search keywords.");
        foreach (var (keyword, template) in SearchEngines)
            if (keyword.EnumerateRunes().Count() is < 1 or > 16 || keyword.StartsWith('=') || keyword.Any(c => char.IsWhiteSpace(c) || char.IsControl(c)) || template is null || template.Length > 512 || !template.Contains("%s") || Core.SearchUrl(template, "test") is null)
                throw new InvalidDataException($"Invalid search keyword \"{keyword}\": use a short word and an http(s) URL containing %s.");
        if (launcherModifiers == settingsModifiers && launcherKey == settingsKey)
            throw new InvalidDataException("Choose different shortcuts for the launcher and Settings.");
    }
}
public sealed class Usage { public long Count { get; set; } public long Last { get; set; } }
/// <param name="Setting">A Windows Settings page rather than an application: hidden from the empty view and ranked after equally good applications.</param>
public sealed record Entry(string Name, string Path, string Description = "", string Fields = "", bool Setting = false);
public sealed class Result(string name, string detail, string kind, string value, bool favorite = false) : INotifyPropertyChanged
{
    public string Name { get; } = name;
    public string Detail { get; } = detail;
    public string Kind { get; } = kind;
    public string Value { get; } = value;
    public bool Favorite { get; } = favorite;
    private ImageSource? icon;
    public ImageSource? Icon
    {
        get => icon;
        set { icon = value; PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(nameof(Icon))); }
    }
    public event PropertyChangedEventHandler? PropertyChanged;
}

public static class Core
{
    public static Settings Copy(Settings settings) => Normalize(JsonSerializer.Deserialize<Settings>(JsonSerializer.Serialize(settings))!);
    private static Settings Normalize(Settings settings)
    {
        settings.SearchEngines = new Dictionary<string, string>(settings.SearchEngines ?? throw new InvalidDataException("Invalid search keywords."), StringComparer.Ordinal);
        settings.Validate();
        settings.Favorites = new HashSet<string>(settings.Favorites, StringComparer.OrdinalIgnoreCase);
        settings.Usage = new Dictionary<string, Usage>(settings.Usage, StringComparer.OrdinalIgnoreCase);
        return settings;
    }
    public static string DataPath => Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData), "LumaLauncher", "native-settings.json");
    public static Settings Load(string path)
    {
        if (!File.Exists(path)) return new Settings();
        if (new FileInfo(path).Length > 4 * 1024 * 1024) throw new InvalidDataException("Settings file exceeds 4 MiB.");
        var settings = JsonSerializer.Deserialize<Settings>(File.ReadAllText(path)) ?? throw new InvalidDataException("Empty settings file.");
        return Normalize(settings);
    }
    public static void Save(string path, Settings settings)
    {
        settings.Validate();
        string dir = Path.GetDirectoryName(path)!;
        Directory.CreateDirectory(dir);
        string tmp = Path.Combine(dir, $".{Guid.NewGuid():N}.tmp");
        try
        {
            using (var file = new FileStream(tmp, FileMode.CreateNew, FileAccess.Write, FileShare.None))
            {
                JsonSerializer.Serialize(file, settings, new JsonSerializerOptions { WriteIndented = true });
                if (file.Length > 4 * 1024 * 1024) throw new InvalidDataException("Settings file exceeds 4 MiB.");
                file.Flush(true);
            }
            File.Move(tmp, path, true);
        }
        finally { if (File.Exists(tmp)) File.Delete(tmp); }
    }
    public static IEnumerable<Entry> Discover()
    {
        foreach (string root in new[] { Environment.GetFolderPath(Environment.SpecialFolder.Programs), Environment.GetFolderPath(Environment.SpecialFolder.CommonPrograms) }.Distinct(StringComparer.OrdinalIgnoreCase))
        {
            if (string.IsNullOrEmpty(root) || !Directory.Exists(root)) continue;
            var options = new EnumerationOptions { RecurseSubdirectories = true, IgnoreInaccessible = true, AttributesToSkip = FileAttributes.ReparsePoint };
            foreach (var path in Directory.EnumerateFiles(root, "*.lnk", options))
                yield return ShellMetadata.Read(path, root);
        }
        foreach (var (name, uri, keywords) in SettingsPages)
            yield return new Entry(name, uri, "Windows Settings", keywords, true);
    }
    // ms-settings: pages. Found by searching ("wifi", "bluetooth") but never listed by default.
    internal static readonly (string Name, string Uri, string Keywords)[] SettingsPages =
    [
        ("Display", "ms-settings:display", "screen resolution brightness monitor night light scale"),
        ("Sound", "ms-settings:sound", "audio volume speaker microphone output input"),
        ("Notifications", "ms-settings:notifications", "alerts focus do not disturb"),
        ("Power & battery", "ms-settings:powersleep", "sleep screen timeout energy saver"),
        ("Storage", "ms-settings:storagesense", "disk space cleanup drives"),
        ("Bluetooth & devices", "ms-settings:bluetooth", "pair wireless"),
        ("Printers & scanners", "ms-settings:printers", "printer scanner"),
        ("Mouse", "ms-settings:mouse", "pointer cursor scroll buttons"),
        ("Touchpad", "ms-settings:devices-touchpad", "gestures trackpad"),
        ("Typing", "ms-settings:typing", "autocorrect spelling keyboard"),
        ("Network & internet", "ms-settings:network", "connection status"),
        ("Wi-Fi", "ms-settings:network-wifi", "wifi wireless network"),
        ("Ethernet", "ms-settings:network-ethernet", "wired network"),
        ("VPN", "ms-settings:network-vpn", "virtual private network"),
        ("Proxy", "ms-settings:network-proxy", "network proxy"),
        ("Airplane mode", "ms-settings:network-airplanemode", "flight radio"),
        ("Mobile hotspot", "ms-settings:network-mobilehotspot", "tethering share internet"),
        ("Personalization", "ms-settings:personalization", "theme appearance"),
        ("Background", "ms-settings:personalization-background", "wallpaper desktop picture"),
        ("Colors", "ms-settings:personalization-colors", "dark mode light mode accent"),
        ("Lock screen", "ms-settings:lockscreen", "login screen"),
        ("Taskbar", "ms-settings:taskbar", "start menu tray"),
        ("Themes", "ms-settings:themes", "personalization"),
        ("Fonts", "ms-settings:fonts", "typeface"),
        ("Apps & features", "ms-settings:appsfeatures", "installed programs uninstall"),
        ("Default apps", "ms-settings:defaultapps", "file associations browser"),
        ("Startup apps", "ms-settings:startupapps", "autostart login boot"),
        ("Accounts", "ms-settings:yourinfo", "user profile"),
        ("Sign-in options", "ms-settings:signinoptions", "password pin windows hello fingerprint"),
        ("Email & accounts", "ms-settings:emailandaccounts", "mail calendar"),
        ("Date & time", "ms-settings:dateandtime", "clock time zone"),
        ("Language & region", "ms-settings:regionlanguage", "keyboard layout locale"),
        ("Accessibility", "ms-settings:easeofaccess", "ease of access narrator magnifier"),
        ("Privacy & security", "ms-settings:privacy", "permissions camera microphone location"),
        ("Windows Update", "ms-settings:windowsupdate", "updates upgrade patches"),
        ("Windows Security", "windowsdefender:", "antivirus defender firewall virus"),
        ("Gaming", "ms-settings:gaming-gamebar", "game bar game mode"),
        ("About", "ms-settings:about", "this pc device name specifications version"),
        ("Clipboard", "ms-settings:clipboard", "history paste"),
        ("Multitasking", "ms-settings:multitasking", "snap windows virtual desktops"),
        ("Recovery", "ms-settings:recovery", "reset this pc restore"),
        ("Troubleshoot", "ms-settings:troubleshoot", "fix problems"),
    ];
    internal static string NormalizeSearch(string text) => UnicodeSearch.Normalize(text);
    public static IEnumerable<Entry> Search(IEnumerable<Entry> entries, string query, Settings s)
    {
        if (query.EnumerateRunes().Count() > 512 || query.Contains('\0')) return Array.Empty<Entry>();
        string q = NormalizeSearch(query.Trim());
        // Dozens of settings pages would otherwise fill the default view.
        if (q.Length == 0) entries = entries.Where(e => !e.Setting);
        (int Tier, int Spread) Rank(Entry e)
        {
            string n = NormalizeSearch(e.Name);
            if (q.Length == 0) return (0, 0);
            if (n == q) return (8, 0);
            if (n.StartsWith(q, StringComparison.Ordinal)) return (7, 0);
            if (n.Contains(" " + q, StringComparison.Ordinal)) return (6, 0);
            if (n.Contains(q, StringComparison.Ordinal)) return (5, 0);
            string acronym = string.Concat(n.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).Select(x => x[0]));
            if (acronym.StartsWith(q, StringComparison.Ordinal)) return (4, 0);
            if (s.Fuzzy)
            {
                int i = 0; string compact = q.Replace(" ", "");
                for (int j = 0; j < n.Length; j++)
                    if (i < compact.Length && n[j] == compact[i] && ++i == compact.Length) return (3, j);
            }
            string fields = NormalizeSearch(e.Fields);
            if (q.Split((char[]?)null, StringSplitOptions.RemoveEmptyEntries).All(fields.Contains)) return (2, 0);
            return s.Descriptions && NormalizeSearch(e.Description).Contains(q, StringComparison.Ordinal) ? (1, 0) : (-1, 0);
        }
        return entries.Select(e => new { Entry = e, Rank = Rank(e), Favorite = s.Favorites.Contains(e.Path), Use = s.Usage.GetValueOrDefault(e.Path) })
            .Where(x => x.Rank.Tier >= 0).OrderByDescending(x => x.Rank.Tier).ThenBy(x => x.Rank.Spread).ThenByDescending(x => x.Favorite)
            .ThenByDescending(x => (long)(2 * Math.Log(1.0 + Math.Max(0, x.Use?.Count ?? 0)))).ThenByDescending(x => x.Use?.Last ?? 0).ThenBy(x => x.Entry.Setting)
            .ThenBy(x => NormalizeSearch(x.Entry.Name), StringComparer.Ordinal).ThenBy(x => x.Entry.Path, StringComparer.OrdinalIgnoreCase).Take(s.Results).Select(x => x.Entry).ToArray();
    }
    public static Dictionary<string, string> DefaultEngines() => new() { ["g"] = "https://www.google.com/search?q=%s", ["ddg"] = "https://duckduckgo.com/?q=%s" };
    /// <summary>One "keyword URL" pair per line, as edited in Settings.</summary>
    public static string EnginesToText(Dictionary<string, string> engines) => string.Concat(engines.OrderBy(e => e.Key, StringComparer.Ordinal).Select(e => $"{e.Key} {e.Value}\n"));
    public static Dictionary<string, string> EnginesFromText(string text)
    {
        var engines = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (string line in text.Split('\n').Select(l => l.Trim()).Where(l => l.Length > 0))
        {
            int split = line.IndexOfAny([' ', '\t']);
            if (split < 0) throw new FormatException($"\"{line}\" needs a keyword and a URL");
            string keyword = line[..split];
            if (!engines.TryAdd(keyword, line[split..].Trim())) throw new FormatException($"\"{keyword}\" is listed twice");
        }
        return engines;
    }
    public static bool IsApplicationQuery(string query, Settings s) => !query.TrimStart().StartsWith('=') && WebUrl(query.Trim(), s) is null;
    /// <summary>Terms are percent-encoded (%20 for spaces) so templates may use them in a path or a query.</summary>
    public static string? SearchUrl(string template, string terms)
    {
        var encoded = new StringBuilder();
        foreach (byte b in Encoding.UTF8.GetBytes(terms))
            if (b < 128 && (char.IsAsciiLetterOrDigit((char)b) || "-_.~".Contains((char)b))) encoded.Append((char)b); else encoded.Append($"%{b:X2}");
        return Uri.TryCreate(template.Replace("%s", encoded.ToString()), UriKind.Absolute, out var uri) && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps) && !string.IsNullOrEmpty(uri.Host) && !template.Any(char.IsControl) ? uri.AbsoluteUri : null;
    }
    public static string? WebUrl(string text, Settings s)
    {
        text = text.Trim();
        int split = text.IndexOfAny([' ', '\t']);
        if (split > 0 && s.SearchEngines.TryGetValue(text[..split], out var template) && text[split..].Trim() is { Length: > 0 } terms) return SearchUrl(template, terms);
        string? scheme = BareAddressScheme(text);
        if (scheme is not null) text = scheme + "://" + text;
        return Uri.TryCreate(text, UriKind.Absolute, out var uri) && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps) && !string.IsNullOrEmpty(uri.Host) && string.IsNullOrEmpty(uri.UserInfo) && !text.Any(c => char.IsControl(c) || char.IsWhiteSpace(c)) ? uri.AbsoluteUri : null;
    }
    // "github.com/rust-lang" or "localhost:3000". Only well-known TLDs qualify so file names stay searches;
    // local development servers get plain HTTP.
    private static string? BareAddressScheme(string text)
    {
        if (text.Contains("://")) return null;
        string authority = text.Split('/', '?', '#')[0];
        string host = authority, port = "";
        int colon = authority.IndexOf(':');
        if (colon >= 0) { host = authority[..colon]; port = authority[(colon + 1)..]; }
        if (!host.All(c => c < 128) || host.Contains('@') || host.Contains(' ') || colon >= 0 && (port.Length is < 1 or > 5 || !port.All(char.IsAsciiDigit))) return null;
        if (host.Equals("localhost", StringComparison.OrdinalIgnoreCase) || host == "127.0.0.1") return colon >= 0 ? "http" : null;
        string[] tlds = ["com", "org", "net", "io", "dev", "app", "ai", "edu", "gov"];
        int dot = host.LastIndexOf('.');
        return dot > 0 && tlds.Contains(host[(dot + 1)..], StringComparer.OrdinalIgnoreCase) ? "https" : null;
    }
    /// <summary>Calculator text for the expression after "=", with an optional "in hex", "in bin" or "in dec" suffix.</summary>
    public static string Calculate(string expression)
    {
        string trimmed = expression.TrimEnd(); string baseName = "dec";
        int at = trimmed.LastIndexOf(" in ", StringComparison.Ordinal);
        if (at >= 0 && trimmed[(at + 4)..].Trim() is "hex" or "bin" or "dec") { baseName = trimmed[(at + 4)..].Trim(); expression = trimmed[..at]; }
        double v = ArithmeticParser.Calculate(expression);
        if (baseName == "dec") return FormatNumber(v);
        if (Math.Truncate(v) != v || Math.Abs(v) > 9007199254740992.0) throw new FormatException("Only whole numbers convert to hex or binary");
        ulong n = (ulong)Math.Abs(v); string sign = v < 0 ? "-" : "";
        return baseName == "hex" ? $"{sign}0x{n:x}" : $"{sign}0b{Convert.ToString((long)n, 2)}";
    }
    internal static string FormatNumber(double value)
    {
        string text = Math.Abs(value) >= 1e15 ? value.ToString("0.##########e0", CultureInfo.InvariantCulture) : value.ToString("0.##########", CultureInfo.InvariantCulture);
        return text == "-0" ? "0" : text; // Tiny negatives round to "-0".
    }
    public static bool SelfTest()
    {
        try
        {
            var s = new Settings();
            SettingsChecks.Run();
            PortChecks.Run();
            PositionChecks.Run();
            s.Favorites.Add("b");
            var items = new[] { new Entry("Code", "a"), new Entry("Code Editor", "b") };
            if (Search(items, "code", s).First().Path != "a" || Search(items, "", s).First().Path != "b") return false;
            if (Calculate("(250 + 75) * 2") != "650" || WebUrl("javascript:alert(1)", s) != null || WebUrl("https://user:pass@example.com", s) != null) return false;
            try { Calculate("1/0"); return false; } catch (FormatException) { } catch (DivideByZeroException) { }
            try { Calculate(new string('(', 25) + "1" + new string(')', 25)); return false; } catch (FormatException) { }
            if (WebUrl("ddg rust & gtk", s) !="https://duckduckgo.com/?q=rust%20%26%20gtk") return false;
            if (!HotkeyParser.TryParse("Ctrl+Alt+Space", out _, out _) || HotkeyParser.TryParse("Space", out _, out _)) return false;
            if (HotkeyParser.TryParse("Ctrl+Alt+999", out _, out _) || HotkeyParser.TryParse("Ctrl+Alt+5", out _, out _) || !HotkeyParser.TryParse("Ctrl+Alt+D5", out _, out _)) return false;
            if (Calculate("-0.00000000001") != "0") return false;
            var oldSettings =JsonSerializer.Deserialize<Settings>("{\"Hotkey\":\"Ctrl+Alt+Space\"}")!;
            oldSettings.Validate();
            if (oldSettings.SettingsHotkey != "Ctrl+Alt+S") return false;
            var duplicate = new Settings { Hotkey = "Ctrl+Alt+S", SettingsHotkey = "ALT+CONTROL+S" };
            try { duplicate.Validate(); return false; } catch (InvalidDataException) { }
            var registered = new Dictionary<int, string>();
            bool Register(string key, int id)
            {
                if (key == "blocked" || registered.ContainsValue(key)) return false;
                registered[id] = key; return true;
            }
            var bindings = new ShortcutBindings(Register, id => registered.Remove(id));
            bindings.Initialize("first", "second");
            if (bindings.TryApply("third", "blocked", out bool restored) || !restored || registered[1] != "first" || registered[2] != "second") return false;
            var previous = bindings.Current;
            if (!bindings.TryApply("second", "first", out _) || registered[1] != "second" || registered[2] != "first") return false;
            if (!bindings.Restore(previous) || registered[1] != "first" || registered[2] != "second") return false;
            var shellIcon = Task.Run(() => ShellIcons.Load(Environment.ProcessPath!)).GetAwaiter().GetResult();
            if (shellIcon is null || !shellIcon.IsFrozen) return false;
            // A shortcut the shell cannot resolve must still get a visible icon rather than an empty slot.
            if (Task.Run(() => ShellIcons.Load(Path.Combine(Path.GetTempPath(), "luma-missing-" + Guid.NewGuid().ToString("N") + ".lnk"))).GetAwaiter().GetResult() is null) return false;
            string dir = Path.Combine(Path.GetTempPath(), "luma-test-" + Guid.NewGuid().ToString("N"));
            try
            {
                string p = Path.Combine(dir, "state.json"); Save(p, s); Save(p, s);
                if (!Load(p).Favorites.Contains("b")) return false;
                File.WriteAllText(p, "broken");
                try { Load(p); return false; } catch (JsonException) { }
                if (File.ReadAllText(p) != "broken") return false;
            }
            finally { if (Directory.Exists(dir)) Directory.Delete(dir, true); }
            return true;
        }
        catch (Exception ex) { Console.Error.WriteLine(ex); return false; }
    }
}
