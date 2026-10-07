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

function Rename-TagIfNeeded {
    param(
        [Parameter(Mandatory = $true)]
        [string] $OldName,
        [Parameter(Mandatory = $true)]
        [string] $NewName
    )

    if (-not (Test-Tag -Name $OldName)) {
        return
    }

    if (Test-Tag -Name $NewName) {
        throw "Cannot rename '$OldName' to '$NewName' because both tags already exist."
    }

    $edit = Invoke-Rinut -Arguments @("tag", "edit", $OldName, "--name", $NewName)
    Require-Success -Result $edit -Action "Renaming tag '$OldName' to '$NewName'"
    Write-Host "Renamed tag: $OldName -> $NewName"
}

function Unlink-TagIfPresent {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Parent,
        [Parameter(Mandatory = $true)]
        [string] $Child
    )

    if (-not (Test-Tag -Name $Parent) -or -not (Test-Tag -Name $Child)) {
        return
    }

    $show = Invoke-Rinut -Arguments @("tag", "show", $Child)
    Require-Success -Result $show -Action "Inspecting tag '$Child'"

    if ($show.Output -contains "Parent: $Parent") {
        $unlink = Invoke-Rinut -Arguments @("tag", "unlink", $Parent, $Child)
        Require-Success -Result $unlink -Action "Unlinking '$Parent' -> '$Child'"
        Write-Host "Unlinked tag hierarchy: $Parent -> $Child"
    }
}

function Delete-TagIfPresent {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    if (-not (Test-Tag -Name $Name)) {
        return
    }

    $delete = Invoke-Rinut -Arguments @("tag", "delete", $Name)
    Require-Success -Result $delete -Action "Deleting tag '$Name'"
    Write-Host "Deleted tag: $Name"
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
    @{ Url = "https://github.com/cloudflare/security-audit-skill"; Tags = @("CS", "skill") },
    @{ Url = "https://github.com/affaan-m/ECC"; Tags = @("CS", "agent-eng", "tool") },
    @{ Url = "https://github.com/alibaba/open-code-review"; Tags = @("CS", "tool") },
    @{ Url = "https://github.com/vectorize-io/hindsight"; Tags = @("CS", "tool") },
    @{ Url = "https://github.com/xai-org/x-algorithm"; Tags = @("CS") },
    @{ Url = "https://github.com/jingyaogong/minimind"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/krahets/hello-algo"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/microsoft/AI-For-Beginners"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/microsoft/ML-For-Beginners"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/microsoft/ai-agents-for-beginners"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/microsoft/mcp-for-beginners"; Tags = @("CS", "mcp", "learning") },
    @{ Url = "https://github.com/nexu-io/open-design"; Tags = @("UI", "tool") },
    @{ Url = "https://github.com/VoltAgent/awesome-design-md"; Tags = @("UI", "reference") },
    @{ Url = "https://github.com/Nutlope/hallmark"; Tags = @("UI", "skill") },
    @{ Url = "https://github.com/penpot/penpot"; Tags = @("UI", "tool") },
    @{ Url = "https://github.com/obra/superpowers"; Tags = @("CS", "agent-eng", "skill") },
    @{ Url = "https://github.com/anthropics/skills/tree/main/skills/mcp-builder"; Tags = @("CS", "mcp", "agent-eng", "skill") },
    @{ Url = "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"; Tags = @("UI", "skill") },
    @{ Url = "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/datawhalechina/happy-llm"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/datawhalechina/self-llm"; Tags = @("CS", "learning") },
    @{ Url = "https://github.com/datawhalechina/hello-agents"; Tags = @("CS", "agent-eng", "learning") },
    @{ Url = "https://github.com/dlvhdr/gh-dash"; Tags = @("github", "tool") },
    @{ Url = "https://github.com/zhaoxuya520/reverse-skill"; Tags = @("CS", "skill") },
    @{ Url = "https://github.com/donnemartin/system-design-primer"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/bilawalsidhu/gods-eye-view"; Tags = @("CS", "tool") },
    @{ Url = "https://github.com/tt-a1i/archify"; Tags = @("CS", "diagram", "skill") },
    @{ Url = "https://github.com/mattpocock/skills"; Tags = @("CS", "agent-eng", "skill") },
    @{ Url = "https://github.com/PanosK92/SpartanEngine"; Tags = @("CS", "game-engine", "learning", "reference") },
    @{ Url = "https://github.com/MrNeRF/LichtFeld-Studio"; Tags = @("CS", "tool") },
    @{ Url = "https://github.com/CyC2018/CS-Notes"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/nilbuild/developer-roadmap"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/codecrafters-io/build-your-own-x"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/freeCodeCamp/freeCodeCamp"; Tags = @("CS", "learning", "course") },
    @{ Url = "https://zh.zlibraryg.ru/"; Tags = @("publishing", "digital-library", "shadow", "service") },
    @{ Url = "https://libgen.ad/"; Tags = @("publishing", "digital-library", "shadow", "service") },
    @{ Url = "https://www.shuge.org/"; Tags = @("humanities", "digital-library", "public-domain", "reference") },
    @{ Url = "https://news.ycombinator.com/"; Tags = @("technology", "technology-news", "community") },
    @{ Url = "https://www.infoq.cn/"; Tags = @("CS", "technology-news", "publication") },
    @{ Url = "https://stackoverflow.com/"; Tags = @("CS", "community", "reference") },
    @{ Url = "https://www.v2ex.com/"; Tags = @("technology", "community") }
)

$renames = @(
    @("computer-science", "CS"),
    @("ui-design", "UI"),
    @("developer-community", "community"),
    @("shadow-library", "shadow"),
    @("architecture-diagram", "diagram")
)

$hierarchy = @(
    @("technology", "CS"),
    @("technology", "UI"),
    @("publishing", "public-domain"),
    @("publishing", "shadow")
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

foreach ($rename in $renames) {
    Rename-TagIfNeeded -OldName $rename[0] -NewName $rename[1]
}

# Remove obsolete deeper taxonomy from older seed revisions.
$obsoleteTags = @(
    "design",
    "AI",
    "ML",
    "LLM",
    "agent",
    "memory",
    "software-eng",
    "system-design",
    "code-review",
    "web",
    "CG",
    "3DGS",
    "3d-recon",
    "gpu-driven",
    "geo-viz",
    "security",
    "reverse",
    "algorithm",
    "data-structure",
    "programming",
    "ai-design",
    "design-system",
    "dev",
    "recommender"
)

foreach ($tag in $obsoleteTags) {
    Delete-TagIfPresent -Name $tag
}

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

foreach ($tag in $allTags) {
    if ($tag.Length -gt 15) {
        throw "Seed tag '$tag' exceeds the 15-character naming guideline."
    }
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
