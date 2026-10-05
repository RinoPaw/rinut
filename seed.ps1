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

function Test-Key {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name
    )

    $show = Invoke-Rinut -Arguments @("key", "show", $Name)
    return $show.ExitCode -eq 0
}

function Ensure-Key {
    param(
        [Parameter(Mandatory = $true)]
        [string] $Name,
        [string] $Type = "taxonomy",
        [string] $Cardinality = "multi"
    )

    $show = Invoke-Rinut -Arguments @("key", "show", $Name)

    if ($show.ExitCode -ne 0) {
        $add = Invoke-Rinut -Arguments @(
            "key", "add", $Name,
            "--type", $Type,
            "--cardinality", $Cardinality
        )
        Require-Success -Result $add -Action "Creating key '$Name'"
        Write-Host "Created key: $Name ($Type, $Cardinality)"
        return
    }

    $text = $show.Output -join "`n"
    $typePattern = "(?m)^Type:\s+$([regex]::Escape($Type))\s*$"
    $cardinalityPattern = "(?m)^Cardinality:\s+$([regex]::Escape($Cardinality))\s*$"

    if ($text -notmatch $typePattern -or $text -notmatch $cardinalityPattern) {
        throw "Existing key '$Name' must be $Type + $Cardinality before this seed can run."
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

function Ensure-Value {
    param(
        [Parameter(Mandatory = $true)]
        [long] $BookmarkId,
        [Parameter(Mandatory = $true)]
        [string] $Key,
        [Parameter(Mandatory = $true)]
        [string] $Value
    )

    $edit = Invoke-Rinut -Arguments @(
        "edit", $BookmarkId.ToString(),
        "--set", "$Key=$Value"
    )
    Require-Success -Result $edit -Action "Setting $Key=$Value on bookmark $BookmarkId"
}

function Remove-All-Values {
    param(
        [Parameter(Mandatory = $true)]
        [long] $BookmarkId,
        [Parameter(Mandatory = $true)]
        [string] $Key
    )

    $edit = Invoke-Rinut -Arguments @(
        "edit", $BookmarkId.ToString(),
        "--unset", $Key
    )
    Require-Success -Result $edit -Action "Removing all $Key values from bookmark $BookmarkId"
}

$bookmarks = @(
    @{
        Url = "https://github.com/cloudflare/security-audit-skill"
        Properties = @{
            domain = @("security")
            concept = @("ai-agent")
            practice = @("code-review")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/affaan-m/ECC"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("ai-agent")
            practice = @("agent-engineering")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/alibaba/open-code-review"
        Properties = @{
            domain = @("software-engineering")
            concept = @("ai-agent")
            practice = @("code-review")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/vectorize-io/hindsight"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("ai-agent", "agent-memory")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/xai-org/x-algorithm"
        Properties = @{
            domain = @("machine-learning")
            concept = @("recommendation-system", "algorithm")
        }
    },
    @{
        Url = "https://github.com/jingyaogong/minimind"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/krahets/hello-algo"
        Properties = @{
            domain = @("computer-science")
            concept = @("algorithm", "data-structure")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/AI-For-Beginners"
        Properties = @{
            domain = @("artificial-intelligence")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/ML-For-Beginners"
        Properties = @{
            domain = @("machine-learning")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/ai-agents-for-beginners"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("ai-agent")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/mcp-for-beginners"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("mcp", "ai-agent")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/nexu-io/open-design"
        Properties = @{
            domain = @("design")
            concept = @("ai-design", "ai-agent")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/VoltAgent/awesome-design-md"
        Properties = @{
            domain = @("design")
            concept = @("design-system", "ai-design")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/Nutlope/hallmark"
        Properties = @{
            domain = @("design")
            concept = @("ai-design", "ui-design")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/penpot/penpot"
        Properties = @{
            domain = @("design")
            concept = @("ui-design")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/obra/superpowers"
        Properties = @{
            domain = @("software-engineering")
            concept = @("ai-agent")
            practice = @("agent-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/anthropics/skills/tree/main/skills/mcp-builder"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("mcp", "ai-agent")
            practice = @("agent-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"
        Properties = @{
            domain = @("design")
            concept = @("ai-design", "ui-design")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/happy-llm"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/self-llm"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/hello-agents"
        Properties = @{
            domain = @("artificial-intelligence")
            concept = @("ai-agent")
            practice = @("agent-engineering")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/dlvhdr/gh-dash"
        Properties = @{
            platform = @("github")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/zhaoxuya520/reverse-skill"
        Properties = @{
            domain = @("security")
            concept = @("ai-agent")
            practice = @("reverse-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/donnemartin/system-design-primer"
        Properties = @{
            domain = @("software-engineering")
            practice = @("system-design")
            purpose = @("learning")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/bilawalsidhu/gods-eye-view"
        Properties = @{
            domain = @("computer-graphics")
            concept = @("geospatial-visualization")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/tt-a1i/archify"
        Properties = @{
            domain = @("software-engineering")
            concept = @("architecture-diagram")
            practice = @("system-design")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/mattpocock/skills"
        Properties = @{
            domain = @("software-engineering")
            practice = @("agent-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/PanosK92/SpartanEngine"
        Properties = @{
            domain = @("computer-graphics")
            concept = @("game-engine", "gpu-driven-rendering")
            purpose = @("learning")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/MrNeRF/LichtFeld-Studio"
        Properties = @{
            domain = @("computer-graphics")
            concept = @("gaussian-splatting", "3d-reconstruction")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/CyC2018/CS-Notes"
        Properties = @{
            domain = @("computer-science")
            purpose = @("learning")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/nilbuild/developer-roadmap"
        Properties = @{
            domain = @("software-engineering")
            purpose = @("learning")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/codecrafters-io/build-your-own-x"
        Properties = @{
            domain = @("computer-science")
            purpose = @("learning")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/freeCodeCamp/freeCodeCamp"
        Properties = @{
            domain = @("software-engineering")
            concept = @("web-development")
            purpose = @("learning")
            kind = @("course")
        }
    }
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

Ensure-Key -Name "domain"
Ensure-Key -Name "concept"
Ensure-Key -Name "practice"
Ensure-Key -Name "platform"
Ensure-Key -Name "purpose"
Ensure-Key -Name "kind"

$legacyTopicExists = Test-Key -Name "topic"

foreach ($bookmark in $bookmarks) {
    $id = Ensure-Bookmark -Url $bookmark.Url

    foreach ($property in $bookmark.Properties.GetEnumerator()) {
        foreach ($value in @($property.Value)) {
            Ensure-Value -BookmarkId $id -Key $property.Key -Value $value
        }
    }

    if ($legacyTopicExists) {
        Remove-All-Values -BookmarkId $id -Key "topic"
    }
}

if ($legacyTopicExists) {
    Write-Host "Legacy key 'topic' is no longer used by seeded bookmarks."
    Write-Host "It is kept to avoid deleting topic data from bookmarks outside this seed."
}

Write-Host ""
Write-Host "Seed complete."
$finalList = Invoke-Rinut -Arguments @("list")
Require-Success -Result $finalList -Action "Listing seeded bookmarks"
$finalList.Output | ForEach-Object { Write-Host $_ }
