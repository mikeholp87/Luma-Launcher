using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Luma;

internal static class ShellIcons
{
    private const uint ShgfiIcon = 0x000000100;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct ShellFileInfo
    {
        public IntPtr Icon;
        public int IconIndex;
        public uint Attributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string DisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)] public string TypeName;
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SHGetFileInfo(string path, uint attributes, out ShellFileInfo info, uint size, uint flags);
    [DllImport("user32.dll")]
    private static extern bool DestroyIcon(IntPtr icon);
    [DllImport("ole32.dll")]
    private static extern int CoInitializeEx(IntPtr reserved, uint concurrencyModel);
    [DllImport("ole32.dll")]
    private static extern void CoUninitialize();

    // The shell resolves a .lnk's own icon, including custom shortcut icons.
    // Call this off the UI thread: shell extensions and network targets can be slow.
    internal static ImageSource? Load(string path)
    {
        int com = CoInitializeEx(IntPtr.Zero, 0); // COINIT_MULTITHREADED
        try
        {
            IntPtr result = SHGetFileInfo(path, 0, out var info, (uint)Marshal.SizeOf<ShellFileInfo>(), ShgfiIcon);
            if (result == IntPtr.Zero || info.Icon == IntPtr.Zero) return null;
            try
            {
                BitmapSource bitmap = Imaging.CreateBitmapSourceFromHIcon(info.Icon, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
                bitmap.Freeze();
                return bitmap;
            }
            finally { DestroyIcon(info.Icon); }
        }
        catch (Exception) { return null; }
        finally { if (com >= 0) CoUninitialize(); }
    }
}
