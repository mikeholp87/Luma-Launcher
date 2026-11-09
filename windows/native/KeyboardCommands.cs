using System.Windows.Input;

namespace Luma;

internal enum PaletteAction { None, Settings, Actions, Jump, Favorite, Forget, Escape, Open, Refresh, Next, Previous }
internal static class KeyboardCommands
{
    internal static (PaletteAction Action, int Index) Resolve(Key key, bool control)
    {
        if (control)
        {
            if (key is Key.OemComma or Key.OemPlus or Key.Add) return (PaletteAction.Settings, 0);
            if (key == Key.Enter) return (PaletteAction.Actions, 0);
            if (key >= Key.D1 && key <= Key.D9) return (PaletteAction.Jump, key - Key.D1);
            if (key >= Key.NumPad1 && key <= Key.NumPad9) return (PaletteAction.Jump, key - Key.NumPad1);
            if (key == Key.D) return (PaletteAction.Favorite, 0);
            if (key == Key.Delete) return (PaletteAction.Forget, 0);
            if (key is Key.J or Key.N) return (PaletteAction.Next, 0);
            if (key is Key.K or Key.P) return (PaletteAction.Previous, 0);
        }
        return (key switch { Key.Escape => PaletteAction.Escape, Key.Enter => PaletteAction.Open, Key.F5 => PaletteAction.Refresh, Key.Down => PaletteAction.Next, Key.Up => PaletteAction.Previous, _ => PaletteAction.None }, 0);
    }
}
