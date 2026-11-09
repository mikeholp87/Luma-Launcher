# Regenerates LumaLauncher.ico from the same design as assets/io.github.luma_launcher.LumaLauncher.svg.
# Usage: powershell -NoProfile -File windows\native\Assets\New-Icon.ps1
Add-Type -AssemblyName System.Drawing
$sizes = 16, 20, 24, 32, 40, 48, 64, 128, 256
$out = Join-Path $PSScriptRoot 'LumaLauncher.ico'

function New-Frame([int]$size) {
    $bitmap = New-Object System.Drawing.Bitmap $size, $size, ([System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.SmoothingMode = 'AntiAlias'; $g.PixelOffsetMode = 'HighQuality'; $g.Clear([System.Drawing.Color]::Transparent)
    $k = $size / 128.0
    # Rounded square: x=4 y=4 w=120 h=120 rx=28
    $r = 28 * $k * 2; $x = 4 * $k; $w = 120 * $k
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($x, $x, $r, $r, 180, 90); $path.AddArc($x + $w - $r, $x, $r, $r, 270, 90)
    $path.AddArc($x + $w - $r, $x + $w - $r, $r, $r, 0, 90); $path.AddArc($x, $x + $w - $r, $r, $r, 90, 90); $path.CloseFigure()
    $g.FillPath((New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#20222b'))), $path)
    # "L" glyph: M39 30 v64 h51 V80 H54 V30 z
    $pts = @(@(39, 30), @(39, 94), @(90, 94), @(90, 80), @(54, 80), @(54, 30)) | ForEach-Object { New-Object System.Drawing.PointF ([single]($_[0] * $k)), ([single]($_[1] * $k)) }
    $g.FillPolygon((New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#8da9ff'))), [System.Drawing.PointF[]]$pts)
    # Highlight dot: circle cx=87 cy=39 r=12
    $g.FillEllipse((New-Object System.Drawing.SolidBrush ([System.Drawing.ColorTranslator]::FromHtml('#edf0f8'))), [single](75 * $k), [single](27 * $k), [single](24 * $k), [single](24 * $k))
    $g.Dispose()
    $stream = New-Object System.IO.MemoryStream
    $bitmap.Save($stream, [System.Drawing.Imaging.ImageFormat]::Png); $bitmap.Dispose()
    , $stream.ToArray()
}

$frames = foreach ($size in $sizes) { , (New-Frame $size) }
$file = [System.IO.File]::Create($out)
$writer = New-Object System.IO.BinaryWriter $file
$writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
$offset = 6 + 16 * $sizes.Count
for ($i = 0; $i -lt $sizes.Count; $i++) {
    $dimension = if ($sizes[$i] -ge 256) { 0 } else { $sizes[$i] }
    $writer.Write([byte]$dimension); $writer.Write([byte]$dimension); $writer.Write([byte]0); $writer.Write([byte]0)
    $writer.Write([uint16]1); $writer.Write([uint16]32); $writer.Write([uint32]$frames[$i].Length); $writer.Write([uint32]$offset)
    $offset += $frames[$i].Length
}
foreach ($frame in $frames) { $writer.Write([byte[]]$frame) }
$writer.Dispose(); $file.Dispose()
Write-Host "Wrote $out"
