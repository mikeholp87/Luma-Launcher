using System.Globalization;
using System.IO;
using System.Text;

namespace Luma;

internal static class UnicodeSearch
{
    // Unicode 17 default full case folding (C/F mappings, excluding Turkic T).
    private static readonly Dictionary<int, string> folding = Load();
    private static Dictionary<int, string> Load()
    {
        using var stream = typeof(UnicodeSearch).Assembly.GetManifestResourceStream("LumaLauncher.Unicode.CaseFolding.txt") ?? throw new InvalidDataException("Missing Unicode case-folding data.");
        using var reader = new StreamReader(stream);
        var result = new Dictionary<int, string>();
        while (reader.ReadLine() is string line)
        {
            var fields = line.Split('#')[0].Split(';', StringSplitOptions.TrimEntries);
            if (fields.Length < 3 || fields[1] is not ("C" or "F")) continue;
            int code = int.Parse(fields[0], NumberStyles.HexNumber, CultureInfo.InvariantCulture);
            result[code] = string.Concat(fields[2].Split(' ', StringSplitOptions.RemoveEmptyEntries).Select(value => char.ConvertFromUtf32(int.Parse(value, NumberStyles.HexNumber, CultureInfo.InvariantCulture))));
        }
        return result;
    }
    internal static string Normalize(string text) => string.Concat(text.EnumerateRunes().Select(r => folding.GetValueOrDefault(r.Value) ?? r.ToString())).Normalize(NormalizationForm.FormKC);
}
