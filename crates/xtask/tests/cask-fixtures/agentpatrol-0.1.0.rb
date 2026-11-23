cask "agentpatrol" do
  version "0.1.0"
  sha256 "eb4a19d1fd08bf78ca0f623ccf73e935f49f7b46e99f76821737289188fdc65a"

  url "https://github.com/byte5digital/agentpatrol/releases/download/v#{version}/AgentPatrol_#{version}_universal.dmg"
  name "AgentPatrol"
  desc "Scans Claude Code and Codex settings and quarantines what a rule proves bad"
  homepage "https://github.com/byte5digital/agentpatrol"

  auto_updates true
  depends_on macos: :sequoia

  app "AgentPatrol.app"
  binary "#{appdir}/AgentPatrol.app/Contents/MacOS/agentpatrol"

  uninstall launchctl: "de.byte5.agentpatrol.daemon",
            quit:      "de.byte5.agentpatrol"

  zap trash: [
    "~/Library/Application Support/de.byte5.agentpatrol",
    "~/Library/Caches/de.byte5.agentpatrol",
    "~/Library/HTTPStorages/de.byte5.agentpatrol",
    "~/Library/Logs/AgentPatrol",
    "~/Library/Preferences/de.byte5.agentpatrol.plist",
    "~/Library/Saved Application State/de.byte5.agentpatrol.savedState",
    "~/Library/WebKit/de.byte5.agentpatrol",
  ]
end
