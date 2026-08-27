param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$ScratchRoot,
    [Parameter(Mandatory = $true)][string]$PythonExe
)

$ErrorActionPreference = 'Stop'
$utf8 = [System.Text.UTF8Encoding]::new($false)
$repo = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$scratch = (Resolve-Path -LiteralPath $ScratchRoot).Path
$tempRoot = (Resolve-Path -LiteralPath $env:TEMP).Path
if (-not $scratch.StartsWith($tempRoot, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Scratch root is not under the resolved TEMP directory: $scratch"
}
if ($scratch -eq $tempRoot) {
    throw 'Scratch root must be a task-specific child of TEMP.'
}

function Assert-CasePath([string]$Path) {
    $full = [System.IO.Path]::GetFullPath($Path)
    if (-not $full.StartsWith($scratch + [System.IO.Path]::DirectorySeparatorChar, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing operation outside audit scratch: $full"
    }
    return $full
}

function New-CaseCopy([string]$Name) {
    $case = Assert-CasePath (Join-Path $scratch $Name)
    if (Test-Path -LiteralPath $case) {
        throw "Case directory already exists: $case"
    }
    New-Item -ItemType Directory -Path $case | Out-Null
    Get-ChildItem -LiteralPath $repo -Force |
        Where-Object { $_.Name -notin @('.git', '.phase1-verification') } |
        Copy-Item -Destination $case -Recurse -Force
    return $case
}

function Remove-CaseCopy([string]$CasePath) {
    $case = Assert-CasePath $CasePath
    if (Test-Path -LiteralPath $case) {
        Remove-Item -LiteralPath $case -Recurse -Force
    }
}

function Write-Utf8([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

function Run-Suite([string]$CasePath) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $PythonExe
    $psi.ArgumentList.Add('-B')
    $psi.ArgumentList.Add('tools/phase1/run_phase1_verification.py')
    $psi.WorkingDirectory = $CasePath
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    [void]$process.Start()
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    $combined = ($stdout + [Environment]::NewLine + $stderr).Trim()
    $failLines = @([regex]::Matches($combined, '(?m)^FAIL .+$') | ForEach-Object Value)
    return [ordered]@{
        exitCode = $process.ExitCode
        detected = ($process.ExitCode -ne 0)
        failLines = $failLines
        output = $combined
    }
}

$mutations = @(
    [ordered]@{ id = 1; name = 'Alter one byte in a controlled file without updating its hash'; apply = {
        param($case)
        $path = Join-Path $case '.editorconfig'
        [System.IO.File]::AppendAllText($path, ' ', $utf8)
    }},
    [ordered]@{ id = 2; name = 'Alter one byte in the frozen research report'; apply = {
        param($case)
        $path = Join-Path $case 'Govs PLC project Research Report.md'
        $bytes = [System.IO.File]::ReadAllBytes($path)
        $bytes[0] = $bytes[0] -bxor 1
        [System.IO.File]::WriteAllBytes($path, $bytes)
    }},
    [ordered]@{ id = 3; name = 'Delete one requirement record from the matrix'; apply = {
        param($case)
        $path = Join-Path $case 'IMPLEMENTATION_MATRIX.json'
        $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        $data.entries = @($data.entries | Select-Object -Skip 1)
        Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
    }},
    [ordered]@{ id = 4; name = 'Change one requirement truth state to VERIFIED'; apply = {
        param($case)
        $path = Join-Path $case 'requirements/phase1-requirements.json'
        $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        $data.requirements[0].truthState = 'VERIFIED'
        Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
    }},
    [ordered]@{ id = 5; name = 'Change one requirement text so it contradicts its directive source'; apply = {
        param($case)
        $path = Join-Path $case 'requirements/phase1-requirements.json'
        $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
        $data.requirements[0].atomicRequirement = 'MUST permit physical industrial communication.'
        Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
    }},
    [ordered]@{ id = 6; name = 'Add https://example.com to a controlled source file'; apply = {
        param($case)
        $path = Join-Path $case 'README.md'
        [System.IO.File]::AppendAllText($path, ([Environment]::NewLine + 'https://example.com' + [Environment]::NewLine), $utf8)
    }},
    [ordered]@{ id = 7; name = 'Add localhost:8080 to a controlled source file'; apply = {
        param($case)
        $path = Join-Path $case 'README.md'
        [System.IO.File]::AppendAllText($path, ([Environment]::NewLine + 'localhost:8080' + [Environment]::NewLine), $utf8)
    }},
    [ordered]@{ id = 8; name = 'Add a real vendor product name to a user-facing string'; apply = {
        param($case)
        $path = Join-Path $case 'README.md'
        [System.IO.File]::AppendAllText($path, ([Environment]::NewLine + 'User-facing mode: Siemens TIA Portal' + [Environment]::NewLine), $utf8)
    }},
    [ordered]@{ id = 9; name = 'Add a network-capable dependency to package.json and Cargo.toml'; apply = {
        param($case)
        $packagePath = Join-Path $case 'package.json'
        $package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
        $package | Add-Member -NotePropertyName dependencies -NotePropertyValue ([ordered]@{ axios = '1.7.9' })
        Write-Utf8 $packagePath (($package | ConvertTo-Json -Depth 20) + [Environment]::NewLine)
        $cargoPath = Join-Path $case 'Cargo.toml'
        [System.IO.File]::AppendAllText($cargoPath, ([Environment]::NewLine + '[dependencies]' + [Environment]::NewLine + 'reqwest = "0.12"' + [Environment]::NewLine), $utf8)
    }},
    [ordered]@{ id = 10; name = 'Introduce a product-root source file with a trivial runtime loop'; apply = {
        param($case)
        $directory = Join-Path $case 'apps/runtime/src'
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
        Write-Utf8 (Join-Path $directory 'main.ts') 'for (;;) { /* runtime loop */ }'
    }},
    [ordered]@{ id = 11; name = 'Close one OPEN risk with no supporting evidence record'; apply = {
        param($case)
        $path = Join-Path $case 'RISK_REGISTER.md'
        $text = [System.IO.File]::ReadAllText($path)
        $tick = [char]96
        $before = $tick + 'OPEN' + $tick + '; architectural rule recorded, implementation and zero-egress evidence not yet available'
        $after = $tick + 'CLOSED' + $tick + '; architectural rule recorded, implementation and zero-egress evidence not yet available'
        if (-not $text.Contains($before)) { throw 'RSK-0001 OPEN text not found' }
        Write-Utf8 $path ($text.Replace($before, $after))
    }},
    [ordered]@{ id = 12; name = 'Remove one ADR file entirely'; apply = {
        param($case)
        $path = Assert-CasePath (Join-Path $case 'ADR/0001-no-physical-industrial-communication.md')
        Remove-Item -LiteralPath $path -Force
    }}
)

$results = [System.Collections.Generic.List[object]]::new()
$baselineCase = New-CaseCopy 'baseline'
try {
    $baselineRun = Run-Suite $baselineCase
    $results.Add([ordered]@{
        id = 0
        mutation = 'Unmodified scratch baseline'
        exitCode = $baselineRun.exitCode
        detected = $baselineRun.detected
        failLines = $baselineRun.failLines
        output = $baselineRun.output
    })
} finally {
    Remove-CaseCopy $baselineCase
}

foreach ($mutation in $mutations) {
    $caseName = 'mutation-{0:d2}' -f $mutation.id
    $case = New-CaseCopy $caseName
    try {
        & $mutation.apply $case
        $run = Run-Suite $case
        $results.Add([ordered]@{
            id = $mutation.id
            mutation = $mutation.name
            exitCode = $run.exitCode
            detected = $run.detected
            failLines = $run.failLines
            output = $run.output
        })
        Write-Output ("M{0:d2} detected={1} exit={2} failures={3}" -f $mutation.id, $run.detected, $run.exitCode, ($run.failLines -join ' || '))
    } finally {
        Remove-CaseCopy $case
    }
}

$resultPath = Join-Path $scratch 'mutation-results.json'
[System.IO.File]::WriteAllText($resultPath, (($results | ConvertTo-Json -Depth 8) + [Environment]::NewLine), $utf8)
Write-Output "RESULT_PATH=$resultPath"
Write-Output "REMAINING_CASE_DIRECTORIES=$(@(Get-ChildItem -LiteralPath $scratch -Directory | Where-Object Name -Like 'mutation-*').Count)"
