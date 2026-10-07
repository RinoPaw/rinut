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


function Ensure-Key {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [Parameter(Mandatory = $true)]
        [ValidateSet("single", "multi")]
        [string] $Cardinality
    )

    $show = Invoke-Rinut -Arguments @("key", $Name)
    if ($show.ExitCode -eq 0) {
        return
    }

    $add = Invoke-Rinut -Arguments @("key", "add", $Name, "--cardinality", $Cardinality)
    Require-Success -Result $add -Action "Creating key '$Name'"
    Write-Host "Created key: $Name ($Cardinality)"
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
        [string] $Url
    )

    $id = Find-BookmarkId -Url $Url

    if ($null -ne $id) {
        Write-Host "Found [$id] $Url"
        return $id
    }

    $add = Invoke-Rinut -Arguments @("add", $Url)
    Require-Success -Result $add -Action "Adding $Url"

    $text = $add.Output -join "`n"
    if ($text -notmatch 'Added \[(\d+)\]') {
        throw "Could not read bookmark ID after adding $Url."
    }

    $id = [long] $Matches[1]
    Write-Host "Added [$id] $Url"
    return $id
}

$bookmarks = @(
    "https://github.com/cloudflare/security-audit-skill",
    "https://github.com/affaan-m/ECC",
    "https://github.com/alibaba/open-code-review",
    "https://github.com/vectorize-io/hindsight",
    "https://github.com/xai-org/x-algorithm",
    "https://github.com/jingyaogong/minimind",
    "https://github.com/krahets/hello-algo",
    "https://github.com/microsoft/AI-For-Beginners",
    "https://github.com/microsoft/ML-For-Beginners",
    "https://github.com/microsoft/ai-agents-for-beginners",
    "https://github.com/microsoft/mcp-for-beginners",
    "https://github.com/nexu-io/open-design",
    "https://github.com/VoltAgent/awesome-design-md",
    "https://github.com/Nutlope/hallmark",
    "https://github.com/penpot/penpot",
    "https://github.com/obra/superpowers",
    "https://github.com/anthropics/skills/tree/main/skills/mcp-builder",
    "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill",
    "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models",
    "https://github.com/datawhalechina/happy-llm",
    "https://github.com/datawhalechina/self-llm",
    "https://github.com/datawhalechina/hello-agents",
    "https://github.com/dlvhdr/gh-dash",
    "https://github.com/zhaoxuya520/reverse-skill",
    "https://github.com/donnemartin/system-design-primer",
    "https://github.com/bilawalsidhu/gods-eye-view",
    "https://github.com/tt-a1i/archify",
    "https://github.com/mattpocock/skills",
    "https://github.com/PanosK92/SpartanEngine",
    "https://github.com/MrNeRF/LichtFeld-Studio",
    "https://github.com/CyC2018/CS-Notes",
    "https://github.com/nilbuild/developer-roadmap",
    "https://github.com/codecrafters-io/build-your-own-x",
    "https://github.com/freeCodeCamp/freeCodeCamp",
    "https://zh.zlibraryg.ru/",
    "https://libgen.ad/",
    "https://www.shuge.org/",
    "https://news.ycombinator.com/",
    "https://www.infoq.cn/",
    "https://stackoverflow.com/",
    "https://www.v2ex.com/"
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

Ensure-Key -Name "Source" -Cardinality "single"
Ensure-Key -Name "Field" -Cardinality "multi"
Ensure-Key -Name "Type" -Cardinality "multi"

# Start from a completely empty tag vocabulary.
$tagList = Invoke-Rinut -Arguments @("tag", "list")
Require-Success -Result $tagList -Action "Listing tags"

foreach ($tag in $tagList.Output) {
    if ([string]::IsNullOrWhiteSpace($tag)) {
        continue
    }

    $delete = Invoke-Rinut -Arguments @("tag", "delete", $tag)
    Require-Success -Result $delete -Action "Deleting tag '$tag'"
    Write-Host "Deleted tag: $tag"
}

foreach ($url in $bookmarks) {
    [void] (Ensure-Bookmark -Url $url)
}

Write-Host ""
Write-Host "Seed complete. All bookmarks are untagged."
$finalList = Invoke-Rinut -Arguments @("list")
Require-Success -Result $finalList -Action "Listing seeded bookmarks"
$finalList.Output | ForEach-Object { Write-Host $_ }
