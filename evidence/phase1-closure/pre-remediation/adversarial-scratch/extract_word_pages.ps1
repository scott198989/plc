param(
    [Parameter(Mandatory = $true)][string]$SourcePath,
    [Parameter(Mandatory = $true)][string]$JsonPath,
    [Parameter(Mandatory = $true)][string]$PdfPath
)

$ErrorActionPreference = 'Stop'
$word = $null
$document = $null
try {
    $resolvedSource = (Resolve-Path -LiteralPath $SourcePath).Path
    $word = New-Object -ComObject Word.Application
    $word.Visible = $false
    $word.DisplayAlerts = 0
    $document = $word.Documents.Open($resolvedSource, $false, $true)
    $pageCount = $document.ComputeStatistics(2)
    $pages = [System.Collections.Generic.List[object]]::new()
    for ($page = 1; $page -le $pageCount; $page++) {
        $start = $document.GoTo(1, 1, $page).Start
        if ($page -lt $pageCount) {
            $finish = $document.GoTo(1, 1, $page + 1).Start
        } else {
            $finish = $document.Content.End
        }
        $text = $document.Range($start, $finish).Text
        $text = $text.Replace([char]13, [char]10).Replace([char]7, [char]9)
        $pages.Add([ordered]@{
            page = $page
            start = $start
            end = $finish
            text = $text
        })
    }
    $document.ExportAsFixedFormat($PdfPath, 17)
    $payload = [ordered]@{
        sourcePath = $resolvedSource
        sourceSha256 = (Get-FileHash -Algorithm SHA256 -LiteralPath $resolvedSource).Hash
        wordVersion = $word.Version
        pageCount = $pageCount
        pages = $pages
    }
    [System.IO.File]::WriteAllText(
        $JsonPath,
        ($payload | ConvertTo-Json -Depth 6),
        [System.Text.UTF8Encoding]::new($false)
    )
    Write-Output "wordVersion=$($word.Version)"
    Write-Output "pageCount=$pageCount"
    Write-Output "sourceSha256=$($payload.sourceSha256)"
    Write-Output "jsonPath=$JsonPath"
    Write-Output "pdfPath=$PdfPath"
} finally {
    if ($null -ne $document) {
        $document.Close($false)
    }
    if ($null -ne $word) {
        $word.Quit()
    }
    if ($null -ne $document) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($document)
    }
    if ($null -ne $word) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($word)
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}
