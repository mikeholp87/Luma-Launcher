$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'Core.ps1')
function Assert-Luma($Condition, [string]$Message) { if (-not $Condition) { throw $Message } }
$state = New-LumaState
$entries = @(
    [pscustomobject]@{ Name = 'Code'; Path = 'code.lnk' },
    [pscustomobject]@{ Name = 'Code Editor'; Path = 'editor.lnk' },
    [pscustomobject]@{ Name = 'Visual Studio Code'; Path = 'vsc.lnk' }
)
$state.favorites = @('editor.lnk')
Assert-Luma ((Get-LumaResults $entries 'code' $state $true 8)[0].Name -eq 'Code') 'Exact matches should beat favorites'
Assert-Luma ((Get-LumaResults $entries '' $state $true 8)[0].Name -eq 'Code Editor') 'Favorites should lead the empty view'
Assert-Luma ((Get-LumaResults $entries 'vsc' $state $true 8)[0].Name -eq 'Visual Studio Code') 'Fuzzy matching should find subsequences'
Assert-Luma (@(Get-LumaResults $entries ('x' * 513) $state $true 8).Count -eq 0) 'Long queries must be rejected'
Assert-Luma ((Get-LumaWebUrl 'ddg rust & gtk') -match 'rust%20%26%20gtk') 'Web query must be encoded'
Assert-Luma (-not (Get-LumaWebUrl 'https://user:pass@example.com')) 'Credential URLs must be rejected'
Assert-Luma (-not (Get-LumaWebUrl 'javascript:alert(1)')) 'Non-web schemes must be rejected'
Assert-Luma ((Get-LumaCalculation '(250 + 75) * 2') -eq '650') 'Arithmetic result is wrong'
foreach ($bad in @('1/0', '1 + system()', ('(' * 25 + '1' + ')' * 25))) {
    $failed = $false
    try { [void](Get-LumaCalculation $bad) } catch { $failed = $true }
    Assert-Luma $failed "Invalid calculation accepted: $bad"
}
$dir = Join-Path ([IO.Path]::GetTempPath()) ('luma-test-' + [guid]::NewGuid().ToString())
try {
    $path = Join-Path $dir 'history.json'
    $state.usage['code.lnk'] = @{ count = 3; last = 123 }
    Save-LumaState $path $state
    Save-LumaState $path $state
    $loaded = Read-LumaState $path
    Assert-Luma ($loaded.favorites -contains 'editor.lnk') 'Favorite did not round-trip'
    Assert-Luma ($loaded.usage['code.lnk'].count -eq 3) 'Usage did not round-trip'
    [IO.File]::WriteAllText($path, 'broken')
    $failed = $false
    try { [void](Read-LumaState $path) } catch { $failed = $true }
    Assert-Luma $failed 'Corrupt history should be rejected'
    Assert-Luma ([IO.File]::ReadAllText($path) -eq 'broken') 'Corrupt history should be preserved'
} finally { if (Test-Path -LiteralPath $dir) { Remove-Item -LiteralPath $dir -Recurse -Force } }
Write-Output 'Windows core tests passed'
