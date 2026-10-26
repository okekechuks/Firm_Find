# Use .NET directly: Windows PowerShell in CI may not expose Get-FileHash.
function Get-ProductionExecutableHash {
    param([Parameter(Mandatory=$true)][string]$LiteralPath)
    $taskAlgorithm = [System.Security.Cryptography.SHA256]::Create()
    try {
        $taskStream = [System.IO.File]::OpenRead($LiteralPath)
        try {
            return [System.BitConverter]::ToString($taskAlgorithm.ComputeHash($taskStream)).Replace('-', '')
        } finally {
            $taskStream.Dispose()
        }
    } finally {
        $taskAlgorithm.Dispose()
    }
}

function Assert-ProductionExecutableUnchanged {
    param(
        [Parameter(Mandatory=$true)][string]$LiteralPath,
        [Parameter(Mandatory=$true)][string]$ExpectedHash
    )
    if (-not [System.IO.File]::Exists($LiteralPath) -or (Get-ProductionExecutableHash $LiteralPath) -ne $ExpectedHash) {
        throw 'Native-test build altered the normal executable. Stop and inspect build configuration.'
    }
}
