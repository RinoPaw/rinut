Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

if (Get-Variable PSNativeCommandUseErrorActionPreference -ErrorAction SilentlyContinue) {
    $PSNativeCommandUseErrorActionPreference = $false
}

function Invoke-Rinut {
    param(
        [Parameter(Mandatory = $true)]
        [string[]] $Arguments
    )

    $output = & cargo run --quiet -- @Arguments 2>&1
    $exitCode = $LASTEXITCODE

    [pscustomobject]@{
        ExitCode = $exitCode
        Output   = @($output | ForEach-Object { $_.ToString() })
    }
}

function Require-Success {
    param(
        [Parameter(Mandatory = $true)]
        $Result,
        [Parameter(Mandatory = $true)]
        [string] $Action
    )

    if ($Result.ExitCode -ne 0) {
        $text = $Result.Output -join [Environment]::NewLine
        throw "$Action failed.$([Environment]::NewLine)$text"
    }
}

function Ensure-TopicKey {
    $show = Invoke-Rinut -Arguments @("key", "show", "topic")

    if ($show.ExitCode -ne 0) {
        $add = Invoke-Rinut -Arguments @(
            "key", "add", "topic",
            "--type", "taxonomy",
            "--cardinality", "multi"
        )
        Require-Success -Result $add -Action "Creating topic key"
        Write-Host "Created key: topic (taxonomy, multi)"
        return
    }

    $text = $show.Output -join "`n"
    if ($text -notmatch '(?m)^Type:\s+taxonomy\s*$' -or
        $text -notmatch '(?m)^Cardinality:\s+multi\s*$') {
        throw "Existing key 'topic' must be taxonomy + multi before this seed can run."
    }
}

function Find-BookmarkId {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Url
    )

    $list = Invoke-Rinut -Arguments @("list")
    Require-Success -Result $list -Action "Listing bookmarks"

    foreach ($line in $list.Output) {
        if ($line -match '^\[(\d+)\]\s+(.+)$' -and $Matches[2] -eq $Url) {
            return [long] $Matches[1]
        }
    }

    return $null
}

function Ensure-Bookmark {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Url,
        [Parameter(Mandatory = $true)]
        [string[]] $Topics
    )

    $id = Find-BookmarkId -Url $Url

    if ($null -eq $id) {
        $add = Invoke-Rinut -Arguments @("add", $Url)
        Require-Success -Result $add -Action "Adding $Url"

        $text = $add.Output -join "`n"
        if ($text -notmatch 'Added \[(\d+)\]') {
            throw "Could not read bookmark ID after adding $Url."
        }

        $id = [long] $Matches[1]
        Write-Host "Added [$id] $Url"
    }
    else {
        Write-Host "Found [$id] $Url"
    }

    foreach ($topic in $Topics) {
        $edit = Invoke-Rinut -Arguments @(
            "edit", $id.ToString(),
            "--set", "topic=$topic"
        )
        Require-Success -Result $edit -Action "Setting topic=$topic on bookmark $id"
    }
}

$bookmarks = @(
    @{
        Url = "https://github.com/cloudflare/security-audit-skill"
        Topics = @("ai-agent", "security", "code-review")
    },
    @{
        Url = "https://github.com/affaan-m/ECC"
        Topics = @("ai-agent", "agent-engineering", "developer-tools")
    },
    @{
        Url = "https://github.com/alibaba/open-code-review"
        Topics = @("ai-agent", "code-review", "developer-tools")
    },
    @{
        Url = "https://github.com/vectorize-io/hindsight"
        Topics = @("ai-agent", "memory", "ai-infrastructure")
    },
    @{
        Url = "https://github.com/xai-org/x-algorithm"
        Topics = @("recommendation-system", "machine-learning", "algorithm")
    },
    @{
        Url = "https://github.com/jingyaogong/minimind"
        Topics = @("large-language-model", "machine-learning", "learning")
    },
    @{
        Url = "https://github.com/krahets/hello-algo"
        Topics = @("algorithm", "data-structure", "learning")
    }
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

Ensure-TopicKey

foreach ($bookmark in $bookmarks) {
    Ensure-Bookmark -Url $bookmark.Url -Topics $bookmark.Topics
}

Write-Host ""
Write-Host "Seed complete."
$finalList = Invoke-Rinut -Arguments @("list")
Require-Success -Result $finalList -Action "Listing seeded bookmarks"
$finalList.Output | ForEach-Object { Write-Host $_ }
