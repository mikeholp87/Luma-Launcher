using System.Windows;
using System.Windows.Threading;
using System.Windows.Controls;
using System.Windows.Media;

namespace Luma;

internal static class PositionChecks
{
    private static void Check(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
    private static void Pump(Window window)
    {
        window.UpdateLayout();
        var frame = new DispatcherFrame();
        window.Dispatcher.BeginInvoke(DispatcherPriority.ContextIdle, new Action(() => frame.Continue = false));
        Dispatcher.PushFrame(frame);
    }
    private static void AssertCentered(PalettePosition position)
    {
        Rect area = position.CurrentWorkArea, bounds = position.CurrentBounds;
        Check(!area.IsEmpty && !bounds.IsEmpty, "Window and monitor bounds must be available.");
        var expected = PalettePosition.CenteredOrigin(area, bounds.Size);
        Check(Math.Abs(bounds.Left - expected.X) <= 1 && Math.Abs(bounds.Top - expected.Y) <= 1, $"Palette must stay centered: {bounds}, work area: {area}.");
    }
    internal static void Run()
    {
        Check(PalettePosition.CenteredOrigin(new Rect(0, 0, 1920, 1040), new Size(680, 400)) == new Point(620, 320), "Center in the usable screen area, excluding the taskbar.");
        Check(PalettePosition.CenteredOrigin(new Rect(-2560, -200, 2560, 1400), new Size(900, 600)) == new Point(-1730, 200), "Secondary monitors may have negative coordinates.");
        Check(PalettePosition.CenteredOrigin(new Rect(100, 50, 800, 600), new Size(1000, 900)) == new Point(100, 50), "Oversized windows must remain on screen.");
        var content = new Border { Height = 400 };
        var window = new Window { Title = "Luma position regression", Width = 680, SizeToContent = SizeToContent.Height, Content = content, WindowStyle = WindowStyle.None, AllowsTransparency = true, Background = Brushes.Transparent, WindowStartupLocation = WindowStartupLocation.Manual, Left = 0, Top = 0, ShowInTaskbar = false };
        using var position = new PalettePosition(window);
        try
        {
            position.BeginShow(); window.Show(); Pump(window);
            AssertCentered(position);
            window.Width = 900; content.Height = 600; Pump(window);
            AssertCentered(position);
            window.Width = 420; content.Height = 200; Pump(window);
            AssertCentered(position);
            window.Hide(); window.Left = 10; window.Top = 10;
            position.BeginShow(); window.Show(); Pump(window);
            AssertCentered(position);
        }
        finally { window.Close(); }
    }
}
