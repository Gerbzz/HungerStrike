# Render only the original synthetic text fixture. All outputs are synthetic test images.
Add-Type -AssemblyName System.Drawing
$source = Join-Path $PSScriptRoot '..\core\tests\fixtures\receipts\R001-purchase.txt'
$out = Join-Path $PSScriptRoot '..\tests\images'
New-Item -ItemType Directory -Path $out -Force | Out-Null
$lines = Get-Content -LiteralPath $source
$font = New-Object System.Drawing.Font('Consolas', 19, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$black = [System.Drawing.Brushes]::Black

function New-ReceiptBitmap([int]$width, [int]$height, [float]$angle, [int]$offsetX) {
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.Clear([System.Drawing.Color]::White)
    $graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $graphics.TranslateTransform([float]$offsetX, 18.0)
    $graphics.RotateTransform($angle)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $graphics.DrawString($lines[$i], $font, $black, 0.0, [float]($i * 31))
    }
    $graphics.Dispose()
    return $bitmap
}

$clean = New-ReceiptBitmap 1240 750 0 18
$clean.Save((Join-Path $out 'synthetic-clean.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$tilted = New-ReceiptBitmap 1240 750 5 45
$tilted.Save((Join-Path $out 'synthetic-tilted.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$cropped = New-ReceiptBitmap 770 600 0 -40
$cropped.Save((Join-Path $out 'synthetic-cropped.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$small = New-Object System.Drawing.Bitmap(310, 188)
$g = [System.Drawing.Graphics]::FromImage($small)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBilinear
$g.DrawImage($clean, 0, 0, 310, 188)
$g.Dispose()
$blurred = New-Object System.Drawing.Bitmap(1240, 750)
$g = [System.Drawing.Graphics]::FromImage($blurred)
$g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBilinear
$g.DrawImage($small, 0, 0, 1240, 750)
$g.Dispose()
$blurred.Save((Join-Path $out 'synthetic-blurred.png'), [System.Drawing.Imaging.ImageFormat]::Png)
$clean.Dispose(); $tilted.Dispose(); $cropped.Dispose(); $small.Dispose(); $blurred.Dispose(); $font.Dispose()
