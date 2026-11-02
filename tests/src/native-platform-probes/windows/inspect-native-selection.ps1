param([Parameter(Mandatory=$true)][long]$WindowHandle,[Parameter(Mandatory=$true)][string]$OutputFile)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
$report=@{apartment=[Threading.Thread]::CurrentThread.GetApartmentState().ToString();stage='started';windowHandle=$WindowHandle}
function Save {$report|ConvertTo-Json|Set-Content $OutputFile -Encoding UTF8}
Save
$root=[System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]::new($WindowHandle))
$condition=[System.Windows.Automation.PropertyCondition]::new([System.Windows.Automation.AutomationElement]::NameProperty,'Home link')
$item=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,$condition)
if($null -eq $item){throw 'Known fixture label absent'}
$pattern=$item.GetCurrentPattern([System.Windows.Automation.TextPattern]::Pattern)
$report.stage='get-selection';Save
$ranges=$pattern.GetSelection();$report.rangeCount=$ranges.Count;$report.stage='get-text';Save
$value='';foreach($range in $ranges){$value+=$range.GetText(-1)}
$report.stage='complete';$report.matchesExpected=$value -ceq 'Home link';$report.textLength=$value.Length;Save
if(!$report.matchesExpected){throw 'Selected fixture text differs'}
