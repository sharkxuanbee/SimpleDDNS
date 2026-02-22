using System.Drawing;
using System.Windows.Forms;

namespace SimpleDDNS.App.Services;

public sealed class TrayIconService : IDisposable
{
    private readonly NotifyIcon _notifyIcon;
    private readonly ToolStripMenuItem _toggleSchedulerMenu;

    public TrayIconService()
    {
        _toggleSchedulerMenu = new ToolStripMenuItem("全局启动");

        _notifyIcon = new NotifyIcon
        {
            Icon = SystemIcons.Application,
            Text = "SimpleDDNS",
            Visible = false
        };

        var menu = new ContextMenuStrip();
        var openItem = new ToolStripMenuItem("打开主界面");
        var runNowItem = new ToolStripMenuItem("立即更新");
        var exitItem = new ToolStripMenuItem("退出");

        openItem.Click += (_, _) => OpenRequested?.Invoke(this, EventArgs.Empty);
        _toggleSchedulerMenu.Click += (_, _) => ToggleSchedulerRequested?.Invoke(this, EventArgs.Empty);
        runNowItem.Click += (_, _) => RunNowRequested?.Invoke(this, EventArgs.Empty);
        exitItem.Click += (_, _) => ExitRequested?.Invoke(this, EventArgs.Empty);

        menu.Items.Add(openItem);
        menu.Items.Add(_toggleSchedulerMenu);
        menu.Items.Add(runNowItem);
        menu.Items.Add(new ToolStripSeparator());
        menu.Items.Add(exitItem);

        _notifyIcon.ContextMenuStrip = menu;
        _notifyIcon.DoubleClick += (_, _) => OpenRequested?.Invoke(this, EventArgs.Empty);
    }

    public event EventHandler? OpenRequested;

    public event EventHandler? ToggleSchedulerRequested;

    public event EventHandler? RunNowRequested;

    public event EventHandler? ExitRequested;

    public void Show()
    {
        _notifyIcon.Visible = true;
    }

    public void UpdateSchedulerState(bool isRunning)
    {
        _toggleSchedulerMenu.Text = isRunning ? "全局停止" : "全局启动";
    }

    public void Dispose()
    {
        _notifyIcon.Visible = false;
        _notifyIcon.Dispose();
    }
}
