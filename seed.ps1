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

function Remove-Value {
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
        "--unset", "$Key=$Value"
    )
    Require-Success -Result $edit -Action "Removing $Key=$Value from bookmark $BookmarkId"
}

$bookmarks = @(
    @{
        Url = "https://github.com/cloudflare/security-audit-skill"
        Properties = @{
            topic = @("ai-agent", "security", "code-review")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/affaan-m/ECC"
        Properties = @{
            topic = @("ai-agent", "agent-engineering")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/alibaba/open-code-review"
        Properties = @{
            topic = @("ai-agent", "code-review")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/vectorize-io/hindsight"
        Properties = @{
            topic = @("ai-agent", "agent-memory", "ai-infrastructure")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/xai-org/x-algorithm"
        Properties = @{
            topic = @("recommendation-system", "machine-learning", "algorithm")
        }
    },
    @{
        Url = "https://github.com/jingyaogong/minimind"
        Properties = @{
            topic = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/krahets/hello-algo"
        Properties = @{
            topic = @("algorithm", "data-structure")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/AI-For-Beginners"
        Properties = @{
            topic = @("artificial-intelligence")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/ML-For-Beginners"
        Properties = @{
            topic = @("machine-learning")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/ai-agents-for-beginners"
        Properties = @{
            topic = @("ai-agent")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/microsoft/mcp-for-beginners"
        Properties = @{
            topic = @("mcp", "ai-agent")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/nexu-io/open-design"
        Properties = @{
            topic = @("ai-design", "ai-agent")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/VoltAgent/awesome-design-md"
        Properties = @{
            topic = @("design-system", "ai-design")
            kind = @("reference")
        }
    },
    @{
        Url = "https://github.com/Nutlope/hallmark"
        Properties = @{
            topic = @("ai-design", "ui-design")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/penpot/penpot"
        Properties = @{
            topic = @("ui-design")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/obra/superpowers"
        Properties = @{
            topic = @("agent-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/anthropics/skills/tree/main/skills/mcp-builder"
        Properties = @{
            topic = @("mcp", "agent-engineering")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/nextlevelbuilder/ui-ux-pro-max-skill"
        Properties = @{
            topic = @("ai-design", "ui-design")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/HandsOnLLM/Hands-On-Large-Language-Models"
        Properties = @{
            topic = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/happy-llm"
        Properties = @{
            topic = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/self-llm"
        Properties = @{
            topic = @("large-language-model")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/datawhalechina/hello-agents"
        Properties = @{
            topic = @("ai-agent", "agent-engineering")
            purpose = @("learning")
        }
    },
    @{
        Url = "https://github.com/dlvhdr/gh-dash"
        Properties = @{
            topic = @("github")
            kind = @("tool")
        }
    },
    @{
        Url = "https://github.com/zhaoxuya520/reverse-skill"
        Properties = @{
            topic = @("reverse-engineering", "security", "ai-agent")
            kind = @("skill")
        }
    },
    @{
        Url = "https://github.com/donnemartin/system-design-primer"
        Properties = @{
            topic = @("system-design")
            purpose = @("learning")
            kind = @("reference")
        }
    }
)

# Values that used to live in topic but now belong to another facet or were too noisy.
$obsoleteTopicValues = @(
    "learning",
    "developer-tools",
    "design-tool",
    "design-skill",
    "terminal",
    "memory"
)

$init = Invoke-Rinut -Arguments @("init")
Require-Success -Result $init -Action "Initializing Rinut"

Ensure-Key -Name "topic"
Ensure-Key -Name "purpose"
Ensure-Key -Name "kind"

foreach ($bookmark in $bookmarks) {
    $id = Ensure-Bookmark -Url $bookmark.Url

    foreach ($property in $bookmark.Properties.GetEnumerator()) {
        foreach ($value in @($property.Value)) {
            Ensure-Value -BookmarkId $id -Key $property.Key -Value $value
        }
    }

    foreach ($value in $obsoleteTopicValues) {
        Remove-Value -BookmarkId $id -Key "topic" -Value $value
    }

    # minimind used to carry a broad parent-like topic that adds little retrieval value.
    if ($bookmark.Url -eq "https://github.com/jingyaogong/minimind") {
        Remove-Value -BookmarkId $id -Key "topic" -Value "machine-learning"
    }
}

Write-Host ""
Write-Host "Seed complete."
$finalList = Invoke-Rinut -Arguments @("list")
Require-Success -Result $finalList -Action "Listing seeded bookmarks"
$finalList.Output | ForEach-Object { Write-Host $_ }
