public class compiler_npe {
    static class MyObj { int val; }

    public static void main(String[] args) {
        System.out.println("Starting...");
        MyObj obj = null;
        obj.val = 5;
        System.out.println("Should not see this.");
    }
}