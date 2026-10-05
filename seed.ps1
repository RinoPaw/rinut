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

function Test-Tag {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    $show = Invoke-Rinut -Arguments @("tag", "show", $Name)
    return $show.ExitCode -eq 0
}

function Ensure-Tag {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    if (Test-Tag -Name $Name) {
        return
    }

    $add = Invoke-Rinut -Arguments @("tag", "add", $Name)
    Require-Success -Result $add -Action "Creating tag '$Name'"
    Write-Host "Created tag: $Name"
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

function Ensure-BookmarkTag {
    param(
        [Parameter(Mandatory = $true)]
        [long] $BookmarkId,
        [Parameter(Mandatory = $true)]
        [string] $Tag
    )

    $edit = Invoke-Rinut -Arguments @("edit", $BookmarkId.ToString(), "--tag", $Tag)
    Require-Success -Result $edit -Action "Tagging bookmark $BookmarkId with '$Tag'"
}

$bookmarks = @(
    @{ Url = "https://github.com/cloudflare/security-audit-skill"; Tags = @("security", "ai-agent", "code-review", "skill") },
    @{ Url = "https://github.com/affaan-m/ECC"; Tags = @("artificial-intelligence", "ai-agent", "agent-engineering", "tool") },
    @{ Url = "https://github.com/alibaba/open-code-review"; Tags = @("software-engineering", "ai-agent", "code-review", "tool") },
    @{ Url = "https://github.com/vectorize-io/hindsight"; Tags = @("artificial-intelligence", "ai-agent", "agent-memory", "tool") },
    @{ Url = "https://github.com/xai-org/x-algorithm"; Tags = @("machine-learning", "recommendation-system", "algorithm") },
    @{ Url = "https://github.com/jingyaogong/minimind"; Tags = @("artificial-intelligence", "large-language-model", "learning") },
    @{ Url = "https://github.com/krahets/hello-algo"; Tags = @("computer-science", "algorithm", "data-structure", "learning") },
    @{ Url = "https://github.com/microsoft/AI-For-Beginners"; Tags = @("artificial-intelligence", "learning") },
    @{ Url = "https://github.com/microsoft/ML-For-Beginners"; Tags = @("machine-learning", "learning") },
    @{ Url = "https://github.com/microsoft/ai-agents-for-beginners"; Tags = @("artificial-intelligence", "ai-agent", "learning") },
    @{ Url = "https://github.com/microsoft/mcp-for-beginners"; Tags = @("artificial-intelligence", "mcp", "ai-agent", "learning") },
    @{ Url = "https://github.com/nexu-io/open-design"; Tags = @("design", "ai-design", "ai-agent", "tool") },
    @{ Url = "https://github.com/VoltAgent/awesome-design-md"; Tags = @("design", "design-system", "ai-design", "reference") },
    @{ Url = "https://github.com/Nutlope/hallmark"; Tags = @("design", "ai-design", "ui-design", "skill") },
    @{ Url = "https://github.com/penpot/penpot"; Tags = @("design", "ui-design", "tool") },
    @{ Url = "https://github.com/obra/superpowers"; Tags = @("software-engineering", "ai-agent", "agent-engineering", "skill") },
    @{ Url = "https://github.com/anthropics/skills/tree/main/skills/mcp-builder"; Tags = @("artificial-intelligence", "mcp", "ai-agent", "agent-engineering", "skill") },
    @{ Url = "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"; Tags = @("design", "ai-design", "ui-design", "skill") },
    @{ Url = "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models"; Tags = @("artificial-intelligence", "large-language-model", "learning") },
    @{ Url = "https://github.com/datawhalechina/happy-llm"; Tags = @("artificial-intelligence", "large-language-model", "learning") },
    @{ Url = "https://github.com/datawhalechina/self-llm"; Tags = @("artificial-intelligence", "large-language-model", "learning") },
    @{ Url = "https://github.com/datawhalechina/hello-agents"; Tags = @("artificial-intelligence", "ai-agent", "agent-engineering", "learning") },
    @{ Url = "https://github.com/dlvhdr/gh-dash"; Tags = @("github", "tool") },
    @{ Url = "https://github.com/zhaoxuya520/reverse-skill"; Tags = @("security", "ai-agent", "reverse-engineering", "skill") },
    @{ Url = "https://github.com/donnemartin/system-design-primer"; Tags = @("software-engineering", "system-design", "learning", "reference") },
    @{ Url = "https://github.com/bilawalsidhu/gods-eye-view"; Tags = @("computer-graphics", "geospatial-visualization", "tool") },
    @{ Url = "https://github.com/tt-a1i/archify"; Tags = @("software-engineering", "architecture-diagram", "system-design", "skill") },
    @{ Url = "https://github.com/mattpocock/skills"; Tags = @("software-engineering", "agent-engineering", "skill") },
    @{ Url = "https://github.com/PanosK92/SpartanEngine"; Tags = @("computer-graphics", "game-engine", "gpu-driven-rendering", "learning", "reference") },
    @{ Url = "https://github.com/MrNeRF/LichtFeld-Studio"; Tags = @("computer-graphics", "gaussian-splatting", "3d-reconstruction", "tool") },
    @{ Url = "https://github.com/CyC2018/CS-Notes"; Tags = @("computer-science", "learning", "reference") },
    @{ Url = "https://github.com/nilbuild/developer-roadmap"; Tags = @("software-engineering", "learning", "reference") },
    @{ Url = "https://github.com/codecrafters-io/build-your-own-x"; Tags = @("computer-science", "learning", "reference") },
    @{ Url = "https://github.com/freeCodeCamp/freeCodeCamp"; Tags = @("software-engineering", "web-development", "learning", "course") },
    @{ Url = "https://zh.zlibraryg.ru/"; Tags = @("publishing", "digital-library", "shadow-library", "service") },
    @{ Url = "https://libgen.ad/"; Tags = @("publishing", "digital-library", "shadow-library", "service") },
    @{ Url = "https://www.shuge.org/"; Tags = @("humanities", "digital-library", "public-domain", "reference") },
    @{ Url = "https://news.ycombinator.com/"; Tags = @("technology", "technology-news", "community") },
    @{ Url = "https://www.infoq.cn/"; Tags = @("software-engineering", "technology-news", "publication") },
    @{ Url = "https://stackoverflow.com/"; Tags = @("software-engineering", "programming", "community", "reference") },
    @{ Url = "https://www.v2ex.com/"; Tags = @("technology", "developer-community", "community") }
)

$hierarchy = @(
    @("technology", "computer-science"),
    @("computer-science", "artificial-intelligence"),
    @("artificial-intelligence", "machine-learning"),
    @("computer-science", "software-engineering"),
    @("computer-science", "computer-graphics"),
    @("computer-science", "security")
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

$allTags = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
foreach ($bookmark in $bookmarks) {
    foreach ($tag in $bookmark.Tags) {
        [void] $allTags.Add($tag)
    }
}
foreach ($edge in $hierarchy) {
    [void] $allTags.Add($edge[0])
    [void] $allTags.Add($edge[1])
}

foreach ($tag in ($allTags | Sort-Object)) {
    Ensure-Tag -Name $tag
}

foreach ($edge in $hierarchy) {
    $link = Invoke-Rinut -Arguments @("tag", "link", $edge[0], $edge[1])
    Require-Success -Result $link -Action "Linking '$($edge[0])' -> '$($edge[1])'"
}

foreach ($bookmark in $bookmarks) {
    $id = Ensure-Bookmark -Url $bookmark.Url
    foreach ($tag in $bookmark.Tags) {
        Ensure-BookmarkTag -BookmarkId $id -Tag $tag
    }
}

Write-Host ""
Write-Host "Seed complete."
$finalList = Invoke-Rinut -Arguments @("list")
Require-Success -Result $finalList -Action "Listing seeded bookmarks"
$finalList.Output | ForEach-Object { Write-Host $_ }
