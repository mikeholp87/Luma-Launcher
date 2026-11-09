using System.Runtime.InteropServices;
using System.Windows.Input;

namespace Luma;

public static class HotkeyParser
{
    public static bool TryParse(string? value, out uint modifiers, out uint virtualKey)
    {
        modifiers = 0; virtualKey = 0;
        if (string.IsNullOrWhiteSpace(value)) return false;
        var parts = value.Split('+', StringSplitOptions.TrimEntries | StringSplitOptions.RemoveEmptyEntries);
        foreach (string part in parts)
        {
            switch (part.ToUpperInvariant())
            {
                case "CTRL": case "CONTROL": if ((modifiers & 2) != 0) return false; modifiers |= 2; break;
                case "ALT": if ((modifiers & 1) != 0) return false; modifiers |= 1; break;
                case "SHIFT": if ((modifiers & 4) != 0) return false; modifiers |= 4; break;
                case "WIN": if ((modifiers & 8) != 0) return false; modifiers |= 8; break;
                default:
                    // Enum.TryParse accepts bare numbers ("5", "999") as undefined or wrong keys; require a named key.
                    if (virtualKey != 0 || int.TryParse(part, out _) || !Enum.TryParse<Key>(part, true, out var key) || !Enum.IsDefined(key) || key is Key.None or Key.System or Key.LeftCtrl or Key.RightCtrl or Key.LeftAlt or Key.RightAlt or Key.LeftShift or Key.RightShift or Key.LWin or Key.RWin) return false;
                    virtualKey = (uint)KeyInterop.VirtualKeyFromKey(key);
                    break;
            }
        }
        return modifiers != 0 && virtualKey != 0;
    }
    public static string FromKey(Key key, ModifierKeys modifiers)
    {
        var parts = new List<string>();
        if (modifiers.HasFlag(ModifierKeys.Control)) parts.Add("Ctrl");
        if (modifiers.HasFlag(ModifierKeys.Alt)) parts.Add("Alt");
        if (modifiers.HasFlag(ModifierKeys.Shift)) parts.Add("Shift");
        if (modifiers.HasFlag(ModifierKeys.Windows)) parts.Add("Win");
        parts.Add(key.ToString());
        return string.Join("+", parts);
    }
}
internal static class Native
{
    [DllImport("user32.dll", SetLastError = true)] internal static extern bool RegisterHotKey(IntPtr hwnd, int id, uint modifiers, uint key);
    [DllImport("user32.dll", SetLastError = true)] internal static extern bool UnregisterHotKey(IntPtr hwnd, int id);
    [DllImport("user32.dll")] private static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] private static extern uint GetWindowThreadProcessId(IntPtr hwnd, out uint processId);
    [DllImport("user32.dll")] private static extern bool AttachThreadInput(uint attach, uint to, bool value);
    [DllImport("user32.dll")] private static extern bool SetForegroundWindow(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern bool BringWindowToTop(IntPtr hwnd);
    [DllImport("user32.dll")] private static extern bool ShowWindow(IntPtr hwnd, int command);
    [DllImport("kernel32.dll")] private static extern uint GetCurrentThreadId();

    // A global hotkey does not always grant foreground rights, so Activate() can
    // merely flash the taskbar button. Share input state with the current
    // foreground thread long enough for SetForegroundWindow to succeed.
    internal static void ForceForeground(IntPtr hwnd)
    {
        if (hwnd == IntPtr.Zero) return;
        IntPtr foreground = GetForegroundWindow();
        if (foreground == hwnd) return;
        uint foregroundThread = foreground == IntPtr.Zero ? 0 : GetWindowThreadProcessId(foreground, out _);
        uint thisThread = GetCurrentThreadId();
        bool attached = foregroundThread != 0 && foregroundThread != thisThread && AttachThreadInput(thisThread, foregroundThread, true);
        try
        {
            ShowWindow(hwnd, 9); // SW_RESTORE
            BringWindowToTop(hwnd);
            SetForegroundWindow(hwnd);
        }
        finally { if (attached) AttachThreadInput(thisThread, foregroundThread, false); }
    }
}

internal sealed record ShortcutState(string Launcher, string Settings, bool LauncherActive, bool SettingsActive);

// Keeps both registrations together so a conflict or failed settings write can
// restore the shortcuts that were active before the attempted change.
internal sealed class ShortcutBindings(Func<string, int, bool> register, Action<int> unregister)
{
    internal ShortcutState Current { get; private set; } = new("", "", false, false);

    internal bool Restore(ShortcutState state)
    {
        if (Current.LauncherActive) unregister(1);
        if (Current.SettingsActive) unregister(2);
        bool launcher = state.LauncherActive && register(state.Launcher, 1);
        bool settings = state.SettingsActive && register(state.Settings, 2);
        Current = state with { LauncherActive = launcher, SettingsActive = settings };
        return launcher == state.LauncherActive && settings == state.SettingsActive;
    }

    internal bool TryApply(string launcher, string settings, out bool restored)
    {
        ShortcutState before = Current;
        if (before.LauncherActive) unregister(1);
        if (before.SettingsActive) unregister(2);
        bool first = register(launcher, 1);
        bool second = first && register(settings, 2);
        Current = new ShortcutState(launcher, settings, first, second);
        if (first && second) { restored = true; return true; }
        restored = Restore(before);
        return false;
    }

    internal void Initialize(string launcher, string settings)
    {
        Current = new ShortcutState(launcher, settings, register(launcher, 1), register(settings, 2));
    }
}
