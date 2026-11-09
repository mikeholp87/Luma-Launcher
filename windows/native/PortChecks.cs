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
        Check(Core.WebUrl("github.com/rust-lang") == "https://github.com/rust-lang", "Bare domains must use HTTPS.");
        foreach (string invalid in new[] { "org.gnome.Nautilus", "notes.md", "firefox", "a b.com", "user@site.com", "javascript:alert(1)", "https://user:pass@example.com" })
            Check(Core.WebUrl(invalid) is null, "Application queries or unsafe URLs must not become URLs: " + invalid);
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
