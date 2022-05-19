# run in VisualStudio Developer Powershell
echo "LIBRARY obs" > obs.def; 
echo "EXPORTS" >> obs.def;
foreach($line in (dumpbin /exports obs.dll | select -skip 19 )) { echo (-split $line | Select-Object -Index 3) >> obs.def};