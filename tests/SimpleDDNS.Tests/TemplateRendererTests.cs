using SimpleDDNS.Core.Services;

namespace SimpleDDNS.Tests;

public class TemplateRendererTests
{
    [Fact]
    public void Render_ReplacesKnownPlaceholders_AndKeepsUnknown()
    {
        var template = "https://example.com/update?host={hostname}&ip={ip}&x={missing}";
        var values = new Dictionary<string, string>
        {
            ["hostname"] = "home.example.com",
            ["ip"] = "203.0.113.7"
        };

        var rendered = TemplateRenderer.Render(template, values);

        Assert.Equal("https://example.com/update?host=home.example.com&ip=203.0.113.7&x={missing}", rendered);
    }
}
