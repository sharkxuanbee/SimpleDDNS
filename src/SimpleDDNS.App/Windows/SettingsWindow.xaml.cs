using System.Windows;
using SimpleDDNS.Core.Models;
using MessageBox = System.Windows.MessageBox;

namespace SimpleDDNS.App.Windows;

public partial class SettingsWindow : Window
{
    public SettingsWindow(AppSettings source)
    {
        InitializeComponent();

        ResultSettings = Clone(source);
        StartWithWindowsCheckBox.IsChecked = ResultSettings.StartWithWindows;
        MinimizeToTrayCheckBox.IsChecked = ResultSettings.MinimizeToTrayOnClose;
        DefaultIntervalTextBox.Text = ResultSettings.DefaultIntervalMinutes.ToString();
        IPv4EndpointsTextBox.Text = string.Join(Environment.NewLine, ResultSettings.IPv4ProbeEndpoints);
        IPv6EndpointsTextBox.Text = string.Join(Environment.NewLine, ResultSettings.IPv6ProbeEndpoints);
    }

    public AppSettings ResultSettings { get; private set; }

    private void SaveButton_OnClick(object sender, RoutedEventArgs e)
    {
        if (!int.TryParse(DefaultIntervalTextBox.Text, out var interval) || interval <= 0)
        {
            MessageBox.Show(this, "默认间隔必须是正整数", "校验失败", MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        var ipv4 = ParseLines(IPv4EndpointsTextBox.Text);
        var ipv6 = ParseLines(IPv6EndpointsTextBox.Text);
        if (ipv4.Count == 0 || ipv6.Count == 0)
        {
            MessageBox.Show(this, "IPv4 和 IPv6 探测源都至少需要 1 个", "校验失败", MessageBoxButton.OK, MessageBoxImage.Warning);
            return;
        }

        ResultSettings.StartWithWindows = StartWithWindowsCheckBox.IsChecked == true;
        ResultSettings.MinimizeToTrayOnClose = MinimizeToTrayCheckBox.IsChecked == true;
        ResultSettings.DefaultIntervalMinutes = interval;
        ResultSettings.IPv4ProbeEndpoints = ipv4;
        ResultSettings.IPv6ProbeEndpoints = ipv6;

        DialogResult = true;
    }

    private void CancelButton_OnClick(object sender, RoutedEventArgs e)
    {
        DialogResult = false;
    }

    private static List<string> ParseLines(string raw)
    {
        return raw
            .Split(new[] { '\r', '\n' }, StringSplitOptions.RemoveEmptyEntries)
            .Select(x => x.Trim())
            .Where(x => !string.IsNullOrWhiteSpace(x))
            .Distinct(StringComparer.OrdinalIgnoreCase)
            .ToList();
    }

    private static AppSettings Clone(AppSettings settings)
    {
        return new AppSettings
        {
            StartWithWindows = settings.StartWithWindows,
            MinimizeToTrayOnClose = settings.MinimizeToTrayOnClose,
            DefaultIntervalMinutes = settings.DefaultIntervalMinutes,
            IPv4ProbeEndpoints = settings.IPv4ProbeEndpoints.ToList(),
            IPv6ProbeEndpoints = settings.IPv6ProbeEndpoints.ToList()
        };
    }
}


