/* Scratch probe: does ConPTY attach a child in this environment? */
#define _WIN32_WINNT 0x0A00
#define WINVER 0x0A00
#include <windows.h>
#include <stdio.h>

static char captured[65536];
static volatile DWORD captured_len = 0;

static DWORD WINAPI reader_thread(LPVOID param) {
    HANDLE pipe = (HANDLE)param;
    char buf[2048];
    DWORD n = 0;
    while (ReadFile(pipe, buf, sizeof(buf), &n, NULL) && n > 0) {
        if (captured_len + n < sizeof(captured)) {
            memcpy(captured + captured_len, buf, n);
            captured_len += n;
        }
    }
    return 0;
}

int main(int argc, char **argv) {
    /* This sandbox runs without a console; give the parent one so ConPTY has
       something to hand the client. */
    AllocConsole();
    freopen("conpty_report.txt", "w", stdout);

    HANDLE inRead, inWrite, outRead, outWrite;
    SECURITY_ATTRIBUTES sa;
    sa.nLength = sizeof(sa);
    sa.lpSecurityDescriptor = NULL;
    sa.bInheritHandle = FALSE;
    CreatePipe(&inRead, &inWrite, &sa, 0);
    CreatePipe(&outRead, &outWrite, &sa, 0);

    HPCON hpc;
    COORD size = { 80, 24 };
    HRESULT hr = CreatePseudoConsole(size, inRead, outWrite, 0, &hpc);
    printf("CreatePseudoConsole hr=0x%08lx\n", (unsigned long)hr);

    STARTUPINFOEXW si;
    ZeroMemory(&si, sizeof(si));
    si.StartupInfo.cb = sizeof(si);
    SIZE_T bytes = 0;
    InitializeProcThreadAttributeList(NULL, 1, 0, &bytes);
    si.lpAttributeList = (LPPROC_THREAD_ATTRIBUTE_LIST)HeapAlloc(GetProcessHeap(), 0, bytes);
    BOOL init = InitializeProcThreadAttributeList(si.lpAttributeList, 1, 0, &bytes);
    PVOID attribute_value = (argc > 1 && argv[1][0] == 'b') ? (PVOID)&hpc : (PVOID)hpc;
    BOOL upd = UpdateProcThreadAttribute(si.lpAttributeList, 0, PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, attribute_value, sizeof(hpc), NULL, NULL);
    printf("attribute=0x%lx value-mode=%c\n", (unsigned long)PROC_THREAD_ATTRIBUTE_PSEUDOCONSOLE, (argc > 1 ? argv[1][0] : 'a'));
    printf("sizeof(STARTUPINFOEXW)=%zu init=%d update=%d\n", sizeof(si), init, upd);

    HANDLE reader = CreateThread(NULL, 0, reader_thread, outRead, 0, NULL);

    wchar_t cmd[] = L"cmd.exe /d /c echo CONPTY-C-MARKER";
    PROCESS_INFORMATION pi;
    ZeroMemory(&pi, sizeof(pi));
    BOOL inherit = (argc > 1 && argv[1][0] == 'c');
    if (argc > 1 && argv[1][0] == 'd') {
        si.StartupInfo.dwFlags = STARTF_USESTDHANDLES;
        si.StartupInfo.hStdInput = NULL;
        si.StartupInfo.hStdOutput = NULL;
        si.StartupInfo.hStdError = NULL;
    }
    BOOL ok = CreateProcessW(NULL, cmd, NULL, NULL, inherit, EXTENDED_STARTUPINFO_PRESENT, NULL, NULL, &si.StartupInfo, &pi);
    printf("CreateProcess ok=%d err=%lu\n", ok, GetLastError());
    fflush(stdout);

    DWORD waited = WaitForSingleObject(pi.hProcess, 5000);
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    printf("wait=%lu exit=%lu\n", waited, code);

    Sleep(1000);
    printf("captured-before-close=%lu\n", captured_len);
    ClosePseudoConsole(hpc);
    Sleep(500);
    printf("--- pty output ---\n");
    fwrite(captured, 1, captured_len, stdout);
    printf("\n--- end (%lu bytes) ---\n", captured_len);
    TerminateThread(reader, 0);
    return 0;
}
