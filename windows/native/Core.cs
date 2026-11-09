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
    public HashSet<string> Favorites { get; set; } = new(StringComparer.OrdinalIgnoreCase);
    public Dictionary<string, Usage> Usage { get; set; } = new(StringComparer.OrdinalIgnoreCase);
    public void Validate()
    {
        if (Theme is not ("Dark" or "Light" or "System") || !Regex.IsMatch(Accent ?? "", "^#[0-9a-fA-F]{6}$") || Width is < 360 or > 1200 || Results is < 1 or > 20 || FontSize is < 12 or > 32 || Favorites is null || Usage is null || !HotkeyParser.TryParse(Hotkey, out uint launcherModifiers, out uint launcherKey) || !HotkeyParser.TryParse(SettingsHotkey, out uint settingsModifiers, out uint settingsKey))
            throw new InvalidDataException("Invalid settings. Check theme, accent, sizes, and shortcut.");
        if (launcherModifiers == settingsModifiers && launcherKey == settingsKey)
            throw new InvalidDataException("Choose different shortcuts for the launcher and Settings.");
    }
}
public sealed class Usage { public long Count { get; set; } public long Last { get; set; } }
public sealed record Entry(string Name, string Path, string Description = "", string Fields = "");
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
    }
    internal static string NormalizeSearch(string text) => UnicodeSearch.Normalize(text);
    public static IEnumerable<Entry> Search(IEnumerable<Entry> entries, string query, Settings s)
    {
        if (query.EnumerateRunes().Count() > 512 || query.Contains('\0')) return Array.Empty<Entry>();
        string q = NormalizeSearch(query.Trim());
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
            .ThenByDescending(x => (long)(2 * Math.Log(1.0 + Math.Max(0, x.Use?.Count ?? 0)))).ThenByDescending(x => x.Use?.Last ?? 0)
            .ThenBy(x => NormalizeSearch(x.Entry.Name), StringComparer.Ordinal).ThenBy(x => x.Entry.Path, StringComparer.OrdinalIgnoreCase).Take(s.Results).Select(x => x.Entry).ToArray();
    }
    public static string? WebUrl(string text)
    {
        text = text.Trim();
        foreach (var (prefix, url) in new[] { ("g ", "https://www.google.com/search?q="), ("ddg ", "https://duckduckgo.com/?q=") })
            if (text.StartsWith(prefix, StringComparison.Ordinal) && text[prefix.Length..].Trim().Length > 0) return url + Uri.EscapeDataString(text[prefix.Length..].Trim());
        string host = text.Split('/', '?', '#')[0];
        string[] tlds = ["com", "org", "net", "io", "dev", "app", "ai", "edu", "gov"];
        if (!text.Contains("://") && host.All(c => c < 128 && !char.IsWhiteSpace(c)) && !host.Contains('@') && !host.Contains(':') && host.Contains('.') && tlds.Contains(host.Split('.')[^1], StringComparer.OrdinalIgnoreCase)) text = "https://" + text;
        return Uri.TryCreate(text, UriKind.Absolute, out var uri) && (uri.Scheme == Uri.UriSchemeHttp || uri.Scheme == Uri.UriSchemeHttps) && !string.IsNullOrEmpty(uri.Host) && string.IsNullOrEmpty(uri.UserInfo) && !text.Any(char.IsControl) ? uri.AbsoluteUri : null;
    }
    public static string Calculate(string expression)
    {
        return FormatNumber(ArithmeticParser.Calculate(expression));
    }
    internal static string FormatNumber(double value)
    {
        string text = value == 0 ? "0" : Math.Abs(value) >= 1e15 ? value.ToString("0.##########e0", CultureInfo.InvariantCulture) : value.ToString("0.##########", CultureInfo.InvariantCulture);
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
            if (Calculate("(250 + 75) * 2") != "650" || WebUrl("javascript:alert(1)") != null || WebUrl("https://user:pass@example.com") != null) return false;
            try { Calculate("1/0"); return false; } catch (FormatException) { } catch (DivideByZeroException) { }
            try { Calculate(new string('(', 25) + "1" + new string(')', 25)); return false; } catch (FormatException) { }
            if (WebUrl("ddg rust & gtk") != "https://duckduckgo.com/?q=rust%20%26%20gtk") return false;
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
