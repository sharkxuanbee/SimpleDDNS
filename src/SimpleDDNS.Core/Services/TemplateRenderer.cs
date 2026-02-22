using System.Text.RegularExpressions;

namespace SimpleDDNS.Core.Services;

public static class TemplateRenderer
{
    private static readonly Regex PlaceholderRegex = new("\\{(?<name>[a-zA-Z0-9_]+)\\}", RegexOptions.Compiled);

    public static string Render(string template, IReadOnlyDictionary<string, string> values)
    {
        ArgumentNullException.ThrowIfNull(template);
        ArgumentNullException.ThrowIfNull(values);

        return PlaceholderRegex.Replace(template, match =>
        {
            var key = match.Groups["name"].Value;
            return values.TryGetValue(key, out var value) ? value : match.Value;
        });
    }
}
