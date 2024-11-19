'use client';
export default function Queue() {
    function joinQueue() {
        console.log("test")
    }

    return (
        <div className="m-2">
            <button className="bg-blue-50 text-black" onClick={joinQueue}>Join queue</button>
        </div>
    )
}
