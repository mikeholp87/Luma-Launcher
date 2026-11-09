using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Threading;

namespace Luma;

// Native window and monitor rectangles use the same screen coordinate space.
// Center the measured window rather than mixing WPF DIPs with monitor pixels.
internal sealed class PalettePosition : IDisposable
{
    private readonly Window window;
    private IntPtr monitor;
    private bool pending, disposed;

    internal PalettePosition(Window window)
    {
        this.window = window;
        window.SizeChanged += SizeChanged;
        window.IsVisibleChanged += VisibilityChanged;
        window.Closed += Closed;
    }

    internal void BeginShow()
    {
        monitor = GetCursorPos(out var pointer) ? MonitorFromPoint(pointer, 1) : IntPtr.Zero;
    }

    internal void Schedule()
    {
        if (disposed || pending || !window.IsVisible || window.Dispatcher.HasShutdownStarted) return;
        pending = true;
        window.Dispatcher.BeginInvoke(DispatcherPriority.Render, new Action(() =>
        {
            pending = false;
            if (!disposed) Center();
        }));
    }

    internal void Center()
    {
        if (disposed || !window.IsVisible) return;
        IntPtr handle = new WindowInteropHelper(window).Handle;
        if (handle == IntPtr.Zero) return;
        var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
        if (monitor == IntPtr.Zero || !GetMonitorInfo(monitor, ref info))
        {
            monitor = MonitorFromWindow(handle, 2);
            if (!GetMonitorInfo(monitor, ref info)) return;
        }
        var dpi = VisualTreeHelper.GetDpi(window);
        window.MaxWidth = Math.Max(1, (info.Work.Right - info.Work.Left) / dpi.DpiScaleX - 32);
        window.MaxHeight = Math.Max(1, Math.Min(880, (info.Work.Bottom - info.Work.Top) / dpi.DpiScaleY - 32));
        if (!GetWindowRect(handle, out var bounds)) return;
        var point = CenteredOrigin(ToRect(info.Work), new Size(bounds.Right - bounds.Left, bounds.Bottom - bounds.Top));
        int left = (int)Math.Round(point.X), top = (int)Math.Round(point.Y);
        if (bounds.Left != left || bounds.Top != top)
            SetWindowPos(handle, IntPtr.Zero, left, top, 0, 0, 0x0001 | 0x0004 | 0x0010); // NOSIZE | NOZORDER | NOACTIVATE
    }

    internal static Point CenteredOrigin(Rect workArea, Size size) => new(
        workArea.Left + Math.Max(0, (workArea.Width - size.Width) / 2),
        workArea.Top + Math.Max(0, (workArea.Height - size.Height) / 2));

    internal Rect CurrentWorkArea
    {
        get
        {
            var info = new MonitorInfo { Size = Marshal.SizeOf<MonitorInfo>() };
            return GetMonitorInfo(monitor, ref info) ? ToRect(info.Work) : Rect.Empty;
        }
    }
    internal Rect CurrentBounds => GetWindowRect(new WindowInteropHelper(window).Handle, out var bounds) ? ToRect(bounds) : Rect.Empty;
    private static Rect ToRect(NativeRect rect) => new(rect.Left, rect.Top, rect.Right - rect.Left, rect.Bottom - rect.Top);
    private void SizeChanged(object sender, SizeChangedEventArgs e) => Schedule();
    private void VisibilityChanged(object sender, DependencyPropertyChangedEventArgs e) => Schedule();
    private void Closed(object? sender, EventArgs e) => Dispose();
    public void Dispose()
    {
        disposed = true;
        window.SizeChanged -= SizeChanged;
        window.IsVisibleChanged -= VisibilityChanged;
        window.Closed -= Closed;
    }

    [StructLayout(LayoutKind.Sequential)] private struct NativePoint { public int X, Y; }
    [StructLayout(LayoutKind.Sequential)] private struct NativeRect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] private struct MonitorInfo { public int Size; public NativeRect Monitor, Work; public uint Flags; }
    [DllImport("user32.dll")] private static extern bool GetCursorPos(out NativePoint point);
    [DllImport("user32.dll")] private static extern IntPtr MonitorFromPoint(NativePoint point, uint flags);
    [DllImport("user32.dll")] private static extern IntPtr MonitorFromWindow(IntPtr window, uint flags);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] private static extern bool GetMonitorInfo(IntPtr monitor, ref MonitorInfo info);
    [DllImport("user32.dll")] private static extern bool GetWindowRect(IntPtr window, out NativeRect rect);
    [DllImport("user32.dll")] private static extern bool SetWindowPos(IntPtr window, IntPtr after, int x, int y, int width, int height, uint flags);
}
