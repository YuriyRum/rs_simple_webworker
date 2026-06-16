A sample project that demonstrates a simple way, how to spawn a Web Worker and execute a function in it from the main thread using async/await syntax

Example:
'''

<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>test</title>
</head>
<body>

<script type="module">

    import init,
    {
        WorkerPool,
    }
    from "./pkg/rs_simple_webworker.js";

    await init();
    const text = 'some test value';

    const pool = WorkerPool.new(10);

    try {
        const result = await pool.get_worker_and_execute(0, "test", function test(text) {
            globalThis.globalState.count++;
            return `Worker response: ${moment().format()} ${globalThis.globalState.count}`;
        }, [text], ["https://momentjs.com/downloads/moment.js"], { count: 0 });
        console.log('result is', result);
    } catch (e) {
        console.error('Error in worker execution:', e);
    }

    try {
        const result2 = await pool.get_worker_and_execute(0, "test", function test(text) {
            globalThis.globalState.count++;
        return `Worker2 repsponse: ${globalThis.globalState.count}`;
        }, [text], ["https://momentjs.com/downloads/moment.js"], { count: 0 });
        console.log('result2 is', result2);
    } catch (e) {
        console.error('Error in worker execution:', e);
    }

    try {
        const result3 = await pool.get_worker_and_execute(3, "test3", function test3(text) {
            globalThis.globalState.count++;
        return `Worker3 response: ${globalThis.globalState.count}`;
        }, [text], ["https://momentjs.com/downloads/moment.js"], { count: 0 });
        console.log('result3 is', result3);
    } catch (e) {
        console.error('Error in worker execution:', e);
    }

</script>

</body>
</html>
'''
