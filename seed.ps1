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
    @{ Url = "https://github.com/cloudflare/security-audit-skill"; Tags = @("security", "agent", "code-review", "skill") },
    @{ Url = "https://github.com/affaan-m/ECC"; Tags = @("AI", "agent", "agent-eng", "tool") },
    @{ Url = "https://github.com/alibaba/open-code-review"; Tags = @("software-eng", "agent", "code-review", "tool") },
    @{ Url = "https://github.com/vectorize-io/hindsight"; Tags = @("AI", "agent", "memory", "tool") },
    @{ Url = "https://github.com/xai-org/x-algorithm"; Tags = @("ML", "algorithm") },
    @{ Url = "https://github.com/jingyaogong/minimind"; Tags = @("AI", "LLM", "learning") },
    @{ Url = "https://github.com/krahets/hello-algo"; Tags = @("CS", "algorithm", "data-structure", "learning") },
    @{ Url = "https://github.com/microsoft/AI-For-Beginners"; Tags = @("AI", "learning") },
    @{ Url = "https://github.com/microsoft/ML-For-Beginners"; Tags = @("ML", "learning") },
    @{ Url = "https://github.com/microsoft/ai-agents-for-beginners"; Tags = @("AI", "agent", "learning") },
    @{ Url = "https://github.com/microsoft/mcp-for-beginners"; Tags = @("AI", "mcp", "agent", "learning") },
    @{ Url = "https://github.com/nexu-io/open-design"; Tags = @("design", "ai-design", "agent", "tool") },
    @{ Url = "https://github.com/VoltAgent/awesome-design-md"; Tags = @("design", "design-system", "ai-design", "reference") },
    @{ Url = "https://github.com/Nutlope/hallmark"; Tags = @("design", "ai-design", "UI", "skill") },
    @{ Url = "https://github.com/penpot/penpot"; Tags = @("design", "UI", "tool") },
    @{ Url = "https://github.com/obra/superpowers"; Tags = @("software-eng", "agent", "agent-eng", "skill") },
    @{ Url = "https://github.com/anthropics/skills/tree/main/skills/mcp-builder"; Tags = @("AI", "mcp", "agent", "agent-eng", "skill") },
    @{ Url = "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"; Tags = @("design", "ai-design", "UI", "skill") },
    @{ Url = "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models"; Tags = @("AI", "LLM", "learning") },
    @{ Url = "https://github.com/datawhalechina/happy-llm"; Tags = @("AI", "LLM", "learning") },
    @{ Url = "https://github.com/datawhalechina/self-llm"; Tags = @("AI", "LLM", "learning") },
    @{ Url = "https://github.com/datawhalechina/hello-agents"; Tags = @("AI", "agent", "agent-eng", "learning") },
    @{ Url = "https://github.com/dlvhdr/gh-dash"; Tags = @("github", "tool") },
    @{ Url = "https://github.com/zhaoxuya520/reverse-skill"; Tags = @("security", "agent", "reverse", "skill") },
    @{ Url = "https://github.com/donnemartin/system-design-primer"; Tags = @("software-eng", "system-design", "learning", "reference") },
    @{ Url = "https://github.com/bilawalsidhu/gods-eye-view"; Tags = @("CG", "geo-viz", "tool") },
    @{ Url = "https://github.com/tt-a1i/archify"; Tags = @("software-eng", "diagram", "system-design", "skill") },
    @{ Url = "https://github.com/mattpocock/skills"; Tags = @("software-eng", "agent-eng", "skill") },
    @{ Url = "https://github.com/PanosK92/SpartanEngine"; Tags = @("CG", "game-engine", "gpu-driven", "learning", "reference") },
    @{ Url = "https://github.com/MrNeRF/LichtFeld-Studio"; Tags = @("CG", "3DGS", "3d-recon", "tool") },
    @{ Url = "https://github.com/CyC2018/CS-Notes"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/nilbuild/developer-roadmap"; Tags = @("software-eng", "learning", "reference") },
    @{ Url = "https://github.com/codecrafters-io/build-your-own-x"; Tags = @("CS", "learning", "reference") },
    @{ Url = "https://github.com/freeCodeCamp/freeCodeCamp"; Tags = @("software-eng", "web", "learning", "course") },
    @{ Url = "https://zh.zlibraryg.ru/"; Tags = @("publishing", "digital-library", "shadow", "service") },
    @{ Url = "https://libgen.ad/"; Tags = @("publishing", "digital-library", "shadow", "service") },
    @{ Url = "https://www.shuge.org/"; Tags = @("humanities", "digital-library", "public-domain", "reference") },
    @{ Url = "https://news.ycombinator.com/"; Tags = @("technology", "technology-news", "community") },
    @{ Url = "https://www.infoq.cn/"; Tags = @("software-eng", "technology-news", "publication") },
    @{ Url = "https://stackoverflow.com/"; Tags = @("software-eng", "programming", "community", "reference") },
    @{ Url = "https://www.v2ex.com/"; Tags = @("technology", "community") }
)

$renames = @(
    @("artificial-intelligence", "AI"),
    @("machine-learning", "ML"),
    @("large-language-model", "LLM"),
    @("computer-science", "CS"),
    @("computer-graphics", "CG"),
    @("software-engineering", "software-eng"),
    @("agent-engineering", "agent-eng"),
    @("reverse-engineering", "reverse-eng"),
    @("architecture-diagram", "arch-diagram"),
    @("developer-community", "dev-community"),
    @("geospatial-visualization", "geo-viz"),
    @("gpu-driven-rendering", "gpu-driven"),
    @("gaussian-splatting", "3DGS"),
    @("3d-reconstruction", "3d-recon"),
    @("ai-agent", "agent"),
    @("agent-memory", "memory"),
    @("system", "design-system"),
    @("ui-design", "UI"),
    @("shadow-library", "shadow"),
    @("arch-diagram", "diagram"),
    @("web-development", "web"),
    @("reverse-eng", "reverse")
)

$hierarchy = @(
    @("technology", "CS"),
    @("technology", "design"),
    @("CS", "AI"),
    @("AI", "ML"),
    @("CS", "software-eng"),
    @("CS", "CG"),
    @("CS", "security"),
    @("CS", "algorithm"),
    @("CS", "data-structure"),
    @("CS", "programming"),
    @("AI", "agent"),
    @("AI", "LLM"),
    @("agent", "memory"),
    @("design", "ai-design"),
    @("design", "UI"),
    @("design", "design-system"),
    @("software-eng", "system-design"),
    @("software-eng", "code-review"),
    @("software-eng", "web"),
    @("security", "reverse"),
    @("CG", "3DGS"),
    @("CG", "3d-recon"),
    @("CG", "gpu-driven"),
    @("CG", "geo-viz"),
    @("publishing", "public-domain"),
    @("publishing", "shadow")
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

foreach ($rename in $renames) {
    Rename-TagIfNeeded -OldName $rename[0] -NewName $rename[1]
}

# Remove hierarchy choices from older seed revisions before applying the new forest.
Unlink-TagIfPresent -Parent "system-design" -Child "diagram"
Unlink-TagIfPresent -Parent "digital-library" -Child "shadow"
Delete-TagIfPresent -Name "dev"
Delete-TagIfPresent -Name "recommender"

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
