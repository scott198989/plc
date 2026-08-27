param(
    [Parameter(Mandatory = $true)][string]$RepositoryRoot,
    [Parameter(Mandatory = $true)][string]$BaselineRef,
    [Parameter(Mandatory = $true)][string]$ScratchRoot,
    [Parameter(Mandatory = $true)][string]$PythonExe
)

$ErrorActionPreference = 'Stop'
$utf8 = [System.Text.UTF8Encoding]::new($false)
$repo = (Resolve-Path -LiteralPath $RepositoryRoot).Path
$python = (Resolve-Path -LiteralPath $PythonExe).Path
$tempRoot = [System.IO.Path]::GetFullPath([System.IO.Path]::GetTempPath()).TrimEnd('\')
$scratch = [System.IO.Path]::GetFullPath($ScratchRoot).TrimEnd('\')
if (-not $scratch.StartsWith($tempRoot + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Scratch root must be a task-specific child of the resolved TEMP directory: $scratch"
}
if (-not (Test-Path -LiteralPath $scratch)) {
    New-Item -ItemType Directory -Path $scratch | Out-Null
}

function Assert-ScratchPath([string]$Path) {
    $full = [System.IO.Path]::GetFullPath($Path)
    if (-not $full.StartsWith($scratch + '\', [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing operation outside mutation scratch: $full"
    }
    return $full
}

function Remove-ScratchChild([string]$Path) {
    $full = Assert-ScratchPath $Path
    if (Test-Path -LiteralPath $full) {
        Remove-Item -LiteralPath $full -Recurse -Force
    }
}

function Write-Utf8([string]$Path, [string]$Content) {
    [System.IO.File]::WriteAllText($Path, $Content, $utf8)
}

function Resolve-GitExe {
    if ($env:PHASE1_GIT -and (Test-Path -LiteralPath $env:PHASE1_GIT)) {
        return (Resolve-Path -LiteralPath $env:PHASE1_GIT).Path
    }
    $command = Get-Command git -ErrorAction SilentlyContinue
    if ($command) {
        return $command.Source
    }
    $bundled = Join-Path $env:USERPROFILE '.cache\codex-runtimes\codex-primary-runtime\dependencies\native\git\cmd\git.exe'
    if (Test-Path -LiteralPath $bundled) {
        return $bundled
    }
    throw 'Git executable is unavailable. Set PHASE1_GIT to the reviewed executable.'
}

function Run-Process(
    [string]$FileName,
    [string[]]$Arguments,
    [string]$WorkingDirectory,
    [int]$TimeoutSeconds = 180
) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $FileName
    foreach ($argument in $Arguments) {
        $psi.ArgumentList.Add($argument)
    }
    $psi.WorkingDirectory = $WorkingDirectory
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $psi
    [void]$process.Start()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        $process.Kill($true)
        $process.WaitForExit()
        throw "Process timed out after $TimeoutSeconds seconds: $FileName"
    }
    $stdout = $stdoutTask.GetAwaiter().GetResult()
    $stderr = $stderrTask.GetAwaiter().GetResult()
    return [ordered]@{
        exitCode = $process.ExitCode
        stdout = $stdout
        stderr = $stderr
        output = ($stdout + [Environment]::NewLine + $stderr).Trim()
    }
}

function Copy-Baseline([string]$Destination) {
    $case = Assert-ScratchPath $Destination
    if (Test-Path -LiteralPath $case) {
        throw "Case directory already exists: $case"
    }
    New-Item -ItemType Directory -Path $case | Out-Null
    Get-ChildItem -LiteralPath $baselineDirectory -Force |
        Copy-Item -Destination $case -Recurse -Force
    return $case
}

function Run-Suite([string]$SubjectRoot) {
    $run = Run-Process $python @(
        '-B',
        'tools/phase1/run_phase1_verification.py',
        '--baseline-manifest',
        $sealedManifest,
        '--baseline-manifest-sha256',
        $manifestSha256,
        '--baseline-commit',
        $baselineCommit
    ) $SubjectRoot
    $failLines = @([regex]::Matches($run.output, '(?m)^FAIL .+$') | ForEach-Object Value)
    $errorLines = @([regex]::Matches($run.output, '(?m)^ERROR .+$') | ForEach-Object Value)
    $crashed = [regex]::IsMatch(
        $run.output,
        '(?im)(^node:|ENOENT|Traceback \(most recent call last\)|Internal verifier|UnhandledPromiseRejection)'
    )
    return [ordered]@{
        exitCode = $run.exitCode
        failLines = $failLines
        errorLines = $errorLines
        crashed = $crashed
        output = $run.output
    }
}

function Hash-Targets([string]$CaseRoot, [string[]]$Targets) {
    $result = [ordered]@{}
    foreach ($target in $Targets) {
        $path = Join-Path $CaseRoot $target
        if (Test-Path -LiteralPath $path -PathType Leaf) {
            $result[$target] = (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash
        } else {
            $result[$target] = $null
        }
    }
    return $result
}

$git = Resolve-GitExe
$resolved = Run-Process $git @('rev-parse', '--verify', "$BaselineRef^{commit}") $repo 30
$baselineCommit = $resolved.stdout.Trim().ToLowerInvariant()
if ($resolved.exitCode -ne 0 -or $baselineCommit -notmatch '^[0-9a-f]{40}$') {
    Write-Error "Baseline ref does not resolve to a commit: $BaselineRef"
    exit 2
}

$archivePath = Assert-ScratchPath (Join-Path $scratch 'baseline.zip')
$baselineDirectory = Assert-ScratchPath (Join-Path $scratch 'baseline')
$sealedDirectory = Assert-ScratchPath (Join-Path $scratch 'sealed')
foreach ($path in @($archivePath, $baselineDirectory, $sealedDirectory)) {
    if (Test-Path -LiteralPath $path) {
        throw "Scratch collision; refusing to overwrite: $path"
    }
}

$archive = Run-Process $git @('archive', '--format=zip', "--output=$archivePath", $baselineCommit) $repo 60
if ($archive.exitCode -ne 0 -or -not (Test-Path -LiteralPath $archivePath)) {
    Write-Error "Unable to archive baseline commit $baselineCommit"
    exit 2
}
$archiveSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $archivePath).Hash
Expand-Archive -LiteralPath $archivePath -DestinationPath $baselineDirectory
New-Item -ItemType Directory -Path $sealedDirectory | Out-Null
$subjectManifest = Join-Path $baselineDirectory 'tests\phase1\trusted-baseline.json'
if (-not (Test-Path -LiteralPath $subjectManifest -PathType Leaf)) {
    Write-Error "Baseline commit does not contain tests/phase1/trusted-baseline.json"
    exit 2
}
$sealedManifest = Join-Path $sealedDirectory 'trusted-baseline.json'
Copy-Item -LiteralPath $subjectManifest -Destination $sealedManifest
$manifestSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $sealedManifest).Hash

$mutations = @(
    [ordered]@{
        id = 1
        name = 'Alter one byte in a controlled file without updating its hash'
        targets = @('.editorconfig')
        expectedCheckId = 'VER-INT-0002'
        expectedDetailPattern = 'SHA-256 mismatch: \.editorconfig'
        apply = {
            param($case)
            [System.IO.File]::AppendAllText((Join-Path $case '.editorconfig'), ' ', $utf8)
        }
    },
    [ordered]@{
        id = 2
        name = 'Alter one byte in the frozen research report'
        targets = @('References for Codex from Scott/Govs PLC project Research Report.md')
        expectedCheckId = 'VER-GOV-0001'
        expectedDetailPattern = 'Govs PLC project Research Report\.md expected .+, got'
        apply = {
            param($case)
            $path = Join-Path $case 'References for Codex from Scott\Govs PLC project Research Report.md'
            $bytes = [System.IO.File]::ReadAllBytes($path)
            $bytes[0] = $bytes[0] -bxor 1
            [System.IO.File]::WriteAllBytes($path, $bytes)
        }
    },
    [ordered]@{
        id = 3
        name = 'Delete one requirement record from the matrix'
        targets = @('IMPLEMENTATION_MATRIX.json')
        expectedCheckId = 'VER-REQ-0002'
        expectedDetailPattern = 'Implementation matrix contains \d+/\d+ entries|identical, unique ID coverage'
        apply = {
            param($case)
            $path = Join-Path $case 'IMPLEMENTATION_MATRIX.json'
            $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
            $data.entries = @($data.entries | Select-Object -Skip 1)
            Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
        }
    },
    [ordered]@{
        id = 4
        name = 'Change one requirement truth state to VERIFIED'
        targets = @('requirements/phase1-requirements.json')
        expectedCheckId = 'VER-REQ-0002'
        expectedDetailPattern = 'contains no self-certified VERIFIED requirements'
        apply = {
            param($case)
            $path = Join-Path $case 'requirements/phase1-requirements.json'
            $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
            $data.requirements[0].truthState = 'VERIFIED'
            Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
        }
    },
    [ordered]@{
        id = 5
        name = 'Change one requirement text so it contradicts its directive source'
        targets = @('requirements/phase1-requirements.json')
        expectedCheckId = 'VER-REQ-0001'
        expectedDetailPattern = 'Deterministic extraction check failed'
        apply = {
            param($case)
            $path = Join-Path $case 'requirements/phase1-requirements.json'
            $data = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
            $data.requirements[0].atomicRequirement = 'MUST permit physical industrial communication.'
            Write-Utf8 $path (($data | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
        }
    },
    [ordered]@{
        id = 6
        name = 'Add https://example.com to a controlled source file'
        targets = @('README.md')
        expectedCheckId = 'VER-OFF-0001'
        expectedDetailPattern = 'Unauthorized external URL in README\.md: https://example\.com'
        apply = {
            param($case)
            [System.IO.File]::AppendAllText(
                (Join-Path $case 'README.md'),
                ([Environment]::NewLine + 'https://example.com' + [Environment]::NewLine),
                $utf8
            )
        }
    },
    [ordered]@{
        id = 7
        name = 'Add localhost:8080 to a controlled source file'
        targets = @('README.md')
        expectedCheckId = 'VER-OFF-0002'
        expectedDetailPattern = 'Unauthorized loopback endpoint in README\.md: localhost:8080'
        apply = {
            param($case)
            [System.IO.File]::AppendAllText(
                (Join-Path $case 'README.md'),
                ([Environment]::NewLine + 'localhost:8080' + [Environment]::NewLine),
                $utf8
            )
        }
    },
    [ordered]@{
        id = 8
        name = 'Add a real vendor product name to a user-facing string'
        targets = @('README.md')
        expectedCheckId = 'VER-BRN-0001'
        expectedDetailPattern = 'Unauthorized vendor-facing text in README\.md: User-facing mode: Siemens TIA Portal'
        apply = {
            param($case)
            [System.IO.File]::AppendAllText(
                (Join-Path $case 'README.md'),
                ([Environment]::NewLine + 'User-facing mode: Siemens TIA Portal' + [Environment]::NewLine),
                $utf8
            )
        }
    },
    [ordered]@{
        id = 9
        name = 'Add a network-capable dependency to package.json and Cargo.toml'
        targets = @('package.json', 'Cargo.toml')
        expectedCheckId = 'VER-DEP-0001'
        expectedDetailPattern = 'Unauthorized network-capable dependency: package\.json dependencies\.axios@1\.7\.9'
        apply = {
            param($case)
            $packagePath = Join-Path $case 'package.json'
            $package = Get-Content -LiteralPath $packagePath -Raw | ConvertFrom-Json
            if ($null -eq $package.dependencies) {
                $package | Add-Member -NotePropertyName dependencies -NotePropertyValue ([ordered]@{})
            }
            $package.dependencies | Add-Member -NotePropertyName axios -NotePropertyValue '1.7.9' -Force
            Write-Utf8 $packagePath (($package | ConvertTo-Json -Depth 100) + [Environment]::NewLine)
            $cargoPath = Join-Path $case 'Cargo.toml'
            [System.IO.File]::AppendAllText(
                $cargoPath,
                ([Environment]::NewLine + "[target.'cfg(any())'.dependencies]" + [Environment]::NewLine + 'reqwest = "0.12"' + [Environment]::NewLine),
                $utf8
            )
        }
    },
    [ordered]@{
        id = 10
        name = 'Introduce a product-root source file with a trivial runtime loop'
        targets = @('apps/runtime/src/main.ts')
        expectedCheckId = 'VER-SCP-0001'
        expectedDetailPattern = 'Unauthorized Phase 1 product-root file: apps/runtime/src/main\.ts'
        apply = {
            param($case)
            $directory = Join-Path $case 'apps\runtime\src'
            New-Item -ItemType Directory -Path $directory -Force | Out-Null
            Write-Utf8 (Join-Path $directory 'main.ts') 'for (;;) { /* runtime loop */ }'
        }
    },
    [ordered]@{
        id = 11
        name = 'Close one OPEN risk with no supporting evidence record'
        targets = @('RISK_REGISTER.md')
        expectedCheckId = 'VER-RSK-0001'
        expectedDetailPattern = 'RSK-0001 is CLOSED but has no approved closureEvidence record'
        apply = {
            param($case)
            $path = Join-Path $case 'RISK_REGISTER.md'
            $text = [System.IO.File]::ReadAllText($path)
            $tick = [char]96
            $before = $tick + 'OPEN' + $tick + '; architectural rule recorded, implementation and zero-egress evidence not yet available'
            $after = $tick + 'CLOSED' + $tick + '; architectural rule recorded, implementation and zero-egress evidence not yet available'
            if (-not $text.Contains($before)) {
                throw 'RSK-0001 OPEN text not found'
            }
            Write-Utf8 $path ($text.Replace($before, $after))
        }
    },
    [ordered]@{
        id = 12
        name = 'Remove one ADR file entirely'
        targets = @('ADR/0001-no-physical-industrial-communication.md')
        expectedCheckId = 'VER-ADR-0001'
        expectedDetailPattern = 'Missing required ADR: ADR/0001-no-physical-industrial-communication\.md'
        apply = {
            param($case)
            Remove-Item -LiteralPath (Join-Path $case 'ADR\0001-no-physical-industrial-communication.md') -Force
        }
    }
)

$results = [System.Collections.Generic.List[object]]::new()
$overallValid = $true
$baselineRun = Run-Suite $baselineDirectory
$baselinePassed = (
    $baselineRun.exitCode -eq 0 -and
    $baselineRun.failLines.Count -eq 0 -and
    $baselineRun.errorLines.Count -eq 0 -and
    -not $baselineRun.crashed
)
$results.Add([ordered]@{
    id = 0
    mutation = 'Unmodified frozen Git-archive baseline'
    baselineCommit = $baselineCommit
    archiveSha256 = $archiveSha256
    manifestSha256 = $manifestSha256
    exitCode = $baselineRun.exitCode
    passed = $baselinePassed
    failLines = $baselineRun.failLines
    errorLines = $baselineRun.errorLines
    crashed = $baselineRun.crashed
    output = $baselineRun.output
})
if (-not $baselinePassed) {
    $overallValid = $false
}

foreach ($mutation in $mutations) {
    $caseName = 'mutation-{0:d2}' -f $mutation.id
    $case = Copy-Baseline (Join-Path $scratch $caseName)
    try {
        $beforeHashes = Hash-Targets $case $mutation.targets
        & $mutation.apply $case
        $afterHashes = Hash-Targets $case $mutation.targets
        $run = Run-Suite $case
        $expectedLinePattern = '^FAIL ' + [regex]::Escape($mutation.expectedCheckId) + ' .*' + $mutation.expectedDetailPattern
        $actualDetectorLines = @($run.failLines | Where-Object { $_ -match $expectedLinePattern })
        $intendedDetected = $actualDetectorLines.Count -gt 0
        $passed = (
            $run.exitCode -eq 1 -and
            $intendedDetected -and
            $run.errorLines.Count -eq 0 -and
            -not $run.crashed
        )
        if (-not $passed) {
            $overallValid = $false
        }
        $results.Add([ordered]@{
            id = $mutation.id
            mutation = $mutation.name
            targets = $mutation.targets
            expectedCheckId = $mutation.expectedCheckId
            expectedDetailPattern = $mutation.expectedDetailPattern
            expectedDetector = $expectedLinePattern
            actualDetectorLines = $actualDetectorLines
            beforeSha256 = $beforeHashes
            afterSha256 = $afterHashes
            exitCode = $run.exitCode
            intendedDetected = $intendedDetected
            passed = $passed
            failLines = $run.failLines
            errorLines = $run.errorLines
            crashed = $run.crashed
            output = $run.output
        })
        Write-Output (
            "M{0:d2} passed={1} exit={2} intended={3} expected={4}" -f
            $mutation.id,
            $passed,
            $run.exitCode,
            $intendedDetected,
            $mutation.expectedCheckId
        )
    } finally {
        Remove-ScratchChild $case
    }
}

$tamperedManifest = Assert-ScratchPath (Join-Path $sealedDirectory 'trusted-baseline-tampered.json')
Copy-Item -LiteralPath $sealedManifest -Destination $tamperedManifest
[System.IO.File]::AppendAllText($tamperedManifest, ' ', $utf8)
$tamper = Run-Process $python @(
    '-B',
    'tools/phase1/run_phase1_verification.py',
    '--baseline-manifest',
    $tamperedManifest,
    '--baseline-manifest-sha256',
    $manifestSha256,
    '--baseline-commit',
    $baselineCommit
) $baselineDirectory
$tamperErrorLines = @([regex]::Matches($tamper.output, '(?m)^ERROR .+$') | ForEach-Object Value)
$tamperCrashed = [regex]::IsMatch(
    $tamper.output,
    '(?im)(^node:|ENOENT|Traceback \(most recent call last\)|Internal verifier|UnhandledPromiseRejection)'
)
$tamperExpectedPattern = 'ERROR VER-INT-0001 Trusted baseline manifest SHA-256 mismatch'
$tamperPassed = (
    $tamper.exitCode -eq 2 -and
    $tamper.output -match $tamperExpectedPattern -and
    -not $tamperCrashed
)
if (-not $tamperPassed) {
    $overallValid = $false
}

$summary = [ordered]@{
    schemaVersion = 1
    baselineCommit = $baselineCommit
    archiveSha256 = $archiveSha256
    manifestSha256 = $manifestSha256
    cleanBaselinePassed = $baselinePassed
    intendedMutationDetections = @($results | Where-Object { $_.id -gt 0 -and $_.passed }).Count
    prescribedMutationCount = 12
    manifestTamperTestPassed = $tamperPassed
    overallPassed = $overallValid
    results = $results
    manifestTamperTest = [ordered]@{
        expectedExitCode = 2
        expectedDetectorPattern = $tamperExpectedPattern
        exitCode = $tamper.exitCode
        passed = $tamperPassed
        actualErrorLines = $tamperErrorLines
        crashed = $tamperCrashed
        output = $tamper.output
    }
}
$resultPath = Assert-ScratchPath (Join-Path $scratch 'mutation-results.json')
[System.IO.File]::WriteAllText(
    $resultPath,
    (($summary | ConvertTo-Json -Depth 20) + [Environment]::NewLine),
    $utf8
)

Remove-ScratchChild $baselineDirectory
Remove-ScratchChild $archivePath
Write-Output "RESULT_PATH=$resultPath"
Write-Output "BASELINE_COMMIT=$baselineCommit"
Write-Output "MANIFEST_SHA256=$manifestSha256"
Write-Output "MUTATION_SCORE=$($summary.intendedMutationDetections)/12"
Write-Output "MANIFEST_TAMPER_TEST=$tamperPassed"
Write-Output "OVERALL_PASS=$overallValid"
Write-Output "REMAINING_CASE_DIRECTORIES=$(@(Get-ChildItem -LiteralPath $scratch -Directory | Where-Object Name -Like 'mutation-*').Count)"

if ($overallValid) {
    exit 0
}
exit 1
