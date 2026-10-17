# Generates clearly labelled SYNTHETIC grocery receipt images for OCR tests.
# Every image carries a visible "SYNTHETIC TEST RECEIPT" banner at the top and
# bottom. Prices, store and items are invented fixture values; nothing here is
# a real transaction. Run from any directory:
#   powershell -NoProfile -ExecutionPolicy Bypass -File scripts\make_synthetic_receipts.ps1
Add-Type -AssemblyName System.Drawing
$out = Join-Path $PSScriptRoot '..\tests\images'
New-Item -ItemType Directory -Path $out -Force | Out-Null

$lines = @(
    'SYNTHETIC TEST RECEIPT',
    'FICTIONAL GROCER - NOT A REAL STORE',
    'FIXTURE ID R101  DATE 2026-10-12',
    '',
    'BANANAS 1.25 LB @ 0.69/LB    0.86',
    'MILK 2% 1 GAL                4.29',
    'EGGS LG 12 CT                3.49',
    'SPINACH 5 OZ                 2.99',
    'RICE 2 LB                    3.79',
    'SUBTOTAL                    15.42',
    'TAX                          0.00',
    'TOTAL                       15.42',
    'SYNTHETIC TEST RECEIPT'
)

$font = New-Object System.Drawing.Font('Consolas', 28, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$ink = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(24, 24, 24))
$red = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(176, 30, 30))

function New-Receipt([int]$width, [int]$height, [float]$angle, [System.Drawing.Color]$background, [bool]$noise) {
    $bitmap = New-Object System.Drawing.Bitmap($width, $height)
    $g = [System.Drawing.Graphics]::FromImage($bitmap)
    $g.Clear($background)
    $g.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
    $g.TranslateTransform(40.0, 30.0)
    $g.RotateTransform($angle)
    $paper = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(252, 251, 246))
    $g.FillRectangle($paper, -16, -14, 640, ($lines.Count * 44) + 28)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $brush = if ($lines[$i] -eq 'SYNTHETIC TEST RECEIPT') { $red } else { $ink }
        $g.DrawString($lines[$i], $font, $brush, 0.0, [float]($i * 44))
    }
    $paper.Dispose()
    $g.Dispose()
    if ($noise) {
        $random = New-Object System.Random(20261012)
        for ($n = 0; $n -lt 9000; $n++) {
            $x = $random.Next(0, $width); $y = $random.Next(0, $height)
            $v = $random.Next(150, 230)
            $bitmap.SetPixel($x, $y, [System.Drawing.Color]::FromArgb($v, $v, $v))
        }
    }
    return $bitmap
}

$clean = New-Receipt 720 680 0 ([System.Drawing.Color]::White) $false
$cleanPath = Join-Path $out 'synthetic-receipt-clean.png'
$clean.Save($cleanPath, [System.Drawing.Imaging.ImageFormat]::Png)

$photo = New-Receipt 760 720 2.5 ([System.Drawing.Color]::FromArgb(120, 112, 100)) $true
$codec = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq 'image/jpeg' }
$params = New-Object System.Drawing.Imaging.EncoderParameters(1)
$params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter([System.Drawing.Imaging.Encoder]::Quality, [long]82)
$photo.Save((Join-Path $out 'synthetic-receipt-photo.jpg'), $codec, $params)

# A deliberately corrupt fixture: a valid PNG signature and header followed by
# truncated image data. Decoders must reject it without crashing.
$bytes = [System.IO.File]::ReadAllBytes($cleanPath)
$cut = [int]($bytes.Length * 0.35)
[System.IO.File]::WriteAllBytes((Join-Path $out 'synthetic-receipt-truncated.png'), $bytes[0..($cut - 1)])

$clean.Dispose(); $photo.Dispose(); $font.Dispose(); $ink.Dispose(); $red.Dispose()
