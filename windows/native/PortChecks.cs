using System.IO;
using System.Windows.Input;

namespace Luma;

internal static class PortChecks
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    internal static void Run()
    {
        var settings = new Settings { Descriptions = false };
        var entries = new[] { new Entry("Studio Tools", "1"), new Entry("Visual Studio Code", "2"), new Entry("Bastudio", "3") };
        Check(Core.Search(entries, "studio", settings).Select(x => x.Path).SequenceEqual(["1", "2", "3"]), "Word starts must beat substrings.");
        entries = [new("Alpha Beta Gamma Delta", "1"), new("Adelphi", "2")];
        Check(Core.Search(entries, "adl", settings).First().Path == "2", "Tighter fuzzy matches must rank first.");
        entries = [new("Visual Studio Code", "1"), new("Very Scattered Characters", "2")];
        settings.Fuzzy = false;
        Check(Core.Search(entries, "vsc", settings).Count() == 2, "Acronyms must work with fuzzy matching disabled.");
        entries = [new("Browser", "1", "Private web surfing", "internet web")];
        Check(Core.Search(entries, "internet web", settings).Any(), "All keyword terms must match.");
        Check(!Core.Search(entries, "surfing", settings).Any(), "Descriptions must be optional.");
        settings.Descriptions = true;
        Check(Core.Search(entries, "surfing", settings).Any(), "Descriptions must be searchable.");
        foreach (var (name, query) in new[] { ("Straße", "STRASSE"), ("Éditeur", "E\u0301DI"), ("ﬃ editor", "FFI"), ("Σςσ", "σσσ"), ("İ", "i\u0307"), ("𐐀", "𐐨") })
            Check(Core.Search([new(name, "1")], query, settings).Any(), "Unicode matching failed: " + name);
        string astral = string.Concat(Enumerable.Repeat("𐐀", 300));
        Check(Core.Search([new(astral, "1")], astral, settings).Any(), "The search limit must count Unicode characters.");
        Check(!Core.Search(entries, new string('x', 513), settings).Any(), "Queries must be bounded.");
        settings.Usage["1"] = new Usage { Count = 5, Last = 100 };
        settings.Usage["2"] = new Usage { Count = 6, Last = 10 };
        Check(Core.Search([new("Alpha", "1"), new("Beta", "2")], "", settings).First().Path == "1", "Recency must break ties within a frequency bucket.");
        settings.Usage["2"].Count = 5000;
        Check(Core.Search([new("Alpha", "1"), new("Beta", "2")], "", settings).First().Path == "2", "High usage counts must not saturate.");
        foreach (var (input, expected) in new[] { ("0.1+0.2", "0.3"), ("1/3", "0.3333333333"), ("-0", "0"), ("-(2+3)*4", "-20"), ("1--2", "3"), ("25\t*\n4", "100"), ("10000000000*10000000000", "1e20") })
            Check(Core.Calculate(input) == expected, "Calculator result failed: " + input);
        foreach (string invalid in new[] { "1/0", "NaN", "1 2", "1e100", "system('dir')", new string('-', 26) + "1" })
        {
            try { Core.Calculate(invalid); throw new Exception("Unsafe or unbounded expression accepted: " + invalid); }
            catch (FormatException) { }
        }
        // Calculator upgrades shared with Linux: powers, percentages, functions, literals and base conversion.
        foreach (var (input, expected) in new[] { ("2^10", "1024"), ("-2^2", "-4"), ("2^3^2", "512"), ("2^-1", "0.5"), ("(1 + 1) ^ 3 * 2", "16"),
            ("50%", "0.5"), ("200 + 10%", "220"), ("200 - 10%", "180"), ("200 * 10%", "20"), ("80 + 50% + 50%", "180"),
            ("sqrt(16) + abs(-2)", "6"), ("round(2.5) + floor(1.9) + ceil(1.1)", "6"), ("log(1000) + ln(e)", "4"), ("cos(0) + sin(0)", "1"), ("0xff + 0b101", "260"),
            ("2 * pi", "6.2831853072"), (" 255 in hex", "0xff"), ("-10 in bin", "-0b1010"), ("0xff in dec", "255") })
            Check(Core.Calculate(input) == expected, "Calculator result failed: " + input + " = " + Core.Calculate(input));
        foreach (string invalid in new[] { "2 ^", "5 % 3", "sqrt 4", "foo(1)", "sqrt(-1)", "0b12", "1.5 in hex", "2 in base64" })
        {
            try { Core.Calculate(invalid); throw new Exception("Invalid expression accepted: " + invalid); }
            catch (FormatException) { }
        }
        // Search keywords.
        var defaults = new Settings();
        Check(Core.WebUrl("github.com/rust-lang", defaults) == "https://github.com/rust-lang", "Bare domains must use HTTPS.");
        Check(Core.WebUrl("ddg rust & gtk", defaults) == "https://duckduckgo.com/?q=rust%20%26%20gtk", "Default keywords must work.");
        var custom = new Settings(); custom.SearchEngines["w"] = "https://en.wikipedia.org/wiki/%s";
        Check(Core.WebUrl("w Rust (language)", custom) == "https://en.wikipedia.org/wiki/Rust%20%28language%29", "Custom keywords must fill their template.");
        Check(Core.WebUrl("g  a/b?c#d ", custom) == "https://www.google.com/search?q=a%2Fb%3Fc%23d", "Terms must be percent-encoded.");
        custom.SearchEngines.Remove("g");
        Check(Core.WebUrl("g linux", custom) is null && Core.WebUrl("w", custom) is null && Core.WebUrl("w   ", custom) is null, "Removed or empty keywords must not match.");
        Check(Core.SearchUrl("javascript:%s", "alert(1)") is null, "Non-HTTP keyword templates must be refused.");
        Check(Core.EnginesFromText(Core.EnginesToText(Core.DefaultEngines())).OrderBy(e => e.Key).SequenceEqual(Core.DefaultEngines().OrderBy(e => e.Key)), "Keyword text must round-trip.");
        Check(Core.EnginesFromText("\n  gh\thttps://github.com/search?q=%s  \n\n")["gh"] == "https://github.com/search?q=%s" && Core.EnginesFromText("").Count == 0, "Keyword text must tolerate whitespace.");
        foreach (string text in new[] { "gh", "g https://a.com/%s\ng https://b.com/%s" })
        {
            try { Core.EnginesFromText(text); throw new Exception("Invalid keyword text accepted: " + text); } catch (FormatException) { }
        }
        foreach (var (keyword, template) in new[] { ("", "https://example.com/?q=%s"), ("two words", "https://example.com/?q=%s"), ("=calc", "https://example.com/?q=%s"), ("averyveryverylongkeyword", "https://example.com/?q=%s"), ("x", "https://example.com/"), ("x", "javascript:alert(%s)"), ("x", "file:///etc/%s") })
        {
            var bad = new Settings(); bad.SearchEngines[keyword] = template;
            try { bad.Validate(); throw new Exception("Invalid keyword accepted: " + keyword + " " + template); } catch (InvalidDataException) { }
        }
        var tooMany = new Settings { SearchEngines = Enumerable.Range(0, 33).ToDictionary(i => "k" + i, _ => "https://example.com/?q=%s") };
        try { tooMany.Validate(); throw new Exception("More than 32 keywords accepted."); } catch (InvalidDataException) { }
        Check(Core.WebUrl("example.com:8080/a", defaults) == "https://example.com:8080/a" && Core.WebUrl("localhost:3000", defaults) == "http://localhost:3000/" && Core.WebUrl("127.0.0.1:8000/x", defaults) == "http://127.0.0.1:8000/x", "Host and port must open without a scheme.");
        Check(!Core.IsApplicationQuery("g linux", defaults) && !Core.IsApplicationQuery(" = 2+2", defaults) && Core.IsApplicationQuery("Calculator", defaults), "Tool queries must not select applications.");
        foreach (string invalid in new[] { "org.gnome.Nautilus", "notes.md", "firefox", "a b.com", "user@site.com", "javascript:alert(1)", "https://user:pass@example.com", "localhost", "mailto:me@example.com", "ftp://example.com", "example.com:port", "example.com:123456", "notes.md:12" })
            Check(Core.WebUrl(invalid, defaults) is null, "Application queries or unsafe URLs must not become URLs: " + invalid);
        // Settings pages are searchable but not listed by default and rank after equal applications.
        var pages = new[] { new Entry("Mousepad", "1"), new Entry("Mouse", "ms-settings:mouse", "", "", true), new Entry("About", "ms-settings:about", "", "", true) };
        Check(Core.Search(pages, "", defaults).Select(x => x.Path).SequenceEqual(["1"]), "The empty view must list applications only.");
        Check(Core.Search(pages, "mouse", defaults).Select(x => x.Path).SequenceEqual(["ms-settings:mouse", "1"]), "Settings pages must be searchable.");
        Check(Core.Search([new Entry("Network Settings", "s", "", "", true), new Entry("Network Tools", "a")], "network", defaults).First().Path == "a", "Applications must rank before equal settings pages.");
        Check(Core.Discover().Any(e => e.Setting), "Settings pages must be indexed.");
        foreach (Key key in new[] { Key.J, Key.N }) Check(KeyboardCommands.Resolve(key, true).Action == PaletteAction.Next, "Next shortcut missing.");
        foreach (Key key in new[] { Key.K, Key.P }) Check(KeyboardCommands.Resolve(key, true).Action == PaletteAction.Previous, "Previous shortcut missing.");
        foreach (Key key in new[] { Key.OemComma, Key.OemPlus, Key.Add }) Check(KeyboardCommands.Resolve(key, true).Action == PaletteAction.Settings, "Settings shortcut missing.");
        for (int i = 0; i < 9; i++)
        {
            Check(KeyboardCommands.Resolve(Key.D1 + i, true) == (PaletteAction.Jump, i), "Number-row quick launch missing.");
            Check(KeyboardCommands.Resolve(Key.NumPad1 + i, true) == (PaletteAction.Jump, i), "Numpad quick launch missing.");
        }
        Check(KeyboardCommands.Resolve(Key.N, false).Action == PaletteAction.None && KeyboardCommands.Resolve(Key.Enter, true).Action == PaletteAction.Actions, "Shortcuts must require Control.");
        Check(MainWindow.Scaled(32, 36, 24) == 61 && MainWindow.Scaled(12, 12, 10) == 10, "Font and icon scaling must match Linux.");
        Check(ResidentCommands.Parse([]) == "--show", "Default command must show the launcher.");
        foreach (string command in ResidentCommands.Supported) Check(ResidentCommands.Parse([command]) == command, "CLI command missing.");
        foreach (string[] invalid in new[] { new[] { "--unknown" }, new[] { "--show", "--hide" } })
        {
            try { ResidentCommands.Parse(invalid); throw new Exception("Invalid command accepted."); } catch (ArgumentException) { }
        }
    }
}
