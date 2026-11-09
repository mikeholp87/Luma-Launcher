using System.IO;
using System.Runtime.InteropServices;
using System.Windows;
using System.Windows.Interop;
using System.Windows.Media;
using System.Windows.Media.Imaging;

namespace Luma;

internal static class ShellIcons
{
    private const uint ShgfiIcon = 0x000000100;
    private const uint ShgsiIcon = 0x000000100;
    private const int SiidApplication = 2;

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct ShellFileInfo
    {
        public IntPtr Icon;
        public int IconIndex;
        public uint Attributes;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string DisplayName;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 80)] public string TypeName;
    }

    [StructLayout(LayoutKind.Sequential, CharSet = CharSet.Unicode)]
    private struct StockIconInfo
    {
        public uint Size;
        public IntPtr Icon;
        public int SysImageIndex;
        public int IconIndex;
        [MarshalAs(UnmanagedType.ByValTStr, SizeConst = 260)] public string Path;
    }

    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern IntPtr SHGetFileInfo(string path, uint attributes, out ShellFileInfo info, uint size, uint flags);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern uint ExtractIconEx(string file, int index, IntPtr[] large, IntPtr[]? small, uint count);
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    private static extern int SHGetStockIconInfo(int id, uint flags, ref StockIconInfo info);
    [DllImport("user32.dll")]
    private static extern bool DestroyIcon(IntPtr icon);

    // Always yields an icon when the shell can supply any: the shortcut's own icon first, then its
    // configured icon location, its target, and finally the stock application icon.
    // The shell is only dependable on STA threads, so extraction runs on a dedicated one.
    internal static ImageSource? Load(string path)
    {
        ImageSource? result = null;
        var worker = new Thread(() => { try { result = LoadCore(path); } catch (Exception) { result = null; } }) { IsBackground = true, Name = "Luma icon loader" };
        worker.SetApartmentState(ApartmentState.STA);
        worker.Start();
        worker.Join();
        return result;
    }

    private static ImageSource? LoadCore(string path)
    {
        var (iconFile, iconIndex, target) = ShellMetadata.ReadIconSource(path);
        bool targetExists = target.Length > 0 && (File.Exists(target) || Directory.Exists(target));
        // A shortcut to a missing program with no icon of its own gets the shell's blank-page icon; skip it.
        bool broken = target.Length > 0 && !targetExists && iconFile.Length == 0;

        if (!broken)
        {
            for (int attempt = 0; attempt < 3; attempt++)
            {
                if (FromFileInfo(path) is { } icon) return icon;
                Thread.Sleep(60 * (attempt + 1)); // Shell extensions can fail transiently under load.
            }
        }
        if (iconFile.Length > 0 && FromFile(Environment.ExpandEnvironmentVariables(iconFile), iconIndex) is { } located) return located;
        if (targetExists && FromFileInfo(target) is { } fromTarget) return fromTarget;
        return FromStock();
    }

    private static ImageSource? FromFileInfo(string path)
    {
        IntPtr result = SHGetFileInfo(path, 0, out var info, (uint)Marshal.SizeOf<ShellFileInfo>(), ShgfiIcon);
        return result == IntPtr.Zero ? null : ToBitmap(info.Icon);
    }

    private static ImageSource? FromFile(string file, int index)
    {
        if (!File.Exists(file)) return null;
        var large = new IntPtr[1];
        return ExtractIconEx(file, index, large, null, 1) == 0 ? null : ToBitmap(large[0]);
    }

    private static ImageSource? FromStock()
    {
        var info = new StockIconInfo { Size = (uint)Marshal.SizeOf<StockIconInfo>() };
        return SHGetStockIconInfo(SiidApplication, ShgsiIcon, ref info) == 0 ? ToBitmap(info.Icon) : null;
    }

    // Consumes the handle. Rejects fully transparent bitmaps so a blank result falls through to the next source.
    private static ImageSource? ToBitmap(IntPtr handle)
    {
        if (handle == IntPtr.Zero) return null;
        try
        {
            BitmapSource source = Imaging.CreateBitmapSourceFromHIcon(handle, Int32Rect.Empty, BitmapSizeOptions.FromEmptyOptions());
            var converted = new FormatConvertedBitmap(source, PixelFormats.Bgra32, null, 0);
            int stride = converted.PixelWidth * 4;
            var pixels = new byte[stride * converted.PixelHeight];
            converted.CopyPixels(pixels, stride, 0);
            bool visible = false;
            for (int i = 3; i < pixels.Length; i += 4) if (pixels[i] != 0) { visible = true; break; }
            if (!visible) return null;
            source.Freeze();
            return source;
        }
        catch (Exception) { return null; }
        finally { DestroyIcon(handle); }
    }
}
